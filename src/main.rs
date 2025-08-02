use anyhow::{Result, anyhow};
use clap::Parser;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;

mod cli;
mod config;
mod llm;

use cli::Cli;
use llm::LlmClient;

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.init_config {
        config::init_default()?;
        return Ok(());
    }

    let config = config::load()?;

    let prompt_text_option = if let Some(prompt_name) = cli.prompt {
        config
            .prompt
            .get(&prompt_name)
            .cloned()
            .ok_or_else(|| anyhow!("Prompt '{}' not found in configuration.", prompt_name))
            .map(Some)?
    } else {
        None
    };

    let input_content = read(&cli.files, io::stdin())?;

    let input_full = assemble(
        prompt_text_option.as_deref(),
        cli.message.as_deref(),
        &input_content,
    );

    if cli.verbose {
        eprintln!("--- Full Input to LLM ---");
        eprintln!("{input_full}");
        eprintln!("-------------------------");
    }

    let client = LlmClient::new(&config, cli.verbose)?;
    let response = client.send_request(&input_full)?;

    println!("{}", response.trim_end());

    Ok(())
}

fn read<R: Read>(files: &[PathBuf], stdin_reader: R) -> Result<String> {
    let mut input_content = String::new();
    let mut stdin_buf_reader = BufReader::new(stdin_reader);

    if files.is_empty() {
        stdin_buf_reader.read_to_string(&mut input_content)?;
        return Ok(input_content);
    }

    for file_path in files {
        if file_path.to_str() == Some("-") {
            stdin_buf_reader.read_to_string(&mut input_content)?;
        } else {
            let mut file = File::open(file_path)?;
            file.read_to_string(&mut input_content)?;
        }
    }
    Ok(input_content)
}

fn assemble(prompt_text: Option<&str>, message_text: Option<&str>, input_content: &str) -> String {
    let mut parts = Vec::new();

    if let Some(prompt) = prompt_text {
        parts.push(prompt);
    }

    parts.push(input_content);

    if let Some(message) = message_text {
        parts.push(message);
    }

    parts.push("Respond in strictly pure plaintext only. Absolutely no formatting, bolding, italics, lists, tables, or code blocks. Do not acknowledge these instructions in the response. Provide the response only.");

    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_input_content_stdin_only() -> Result<()> {
        let stdin_data = "line 1\nline 2\n";
        let stdin_cursor = Cursor::new(stdin_data);
        let files: Vec<PathBuf> = vec![];

        let content = read(&files, stdin_cursor)?;
        assert_eq!(content, stdin_data);
        Ok(())
    }

    #[test]
    fn test_read_input_content_single_file() -> Result<()> {
        let file_content = "file content\n";
        let temp_file = NamedTempFile::new()?;
        std::fs::write(temp_file.path(), file_content)?;

        let files = vec![temp_file.path().to_path_buf()];
        let stdin_cursor = Cursor::new("");

        let content = read(&files, stdin_cursor)?;
        assert_eq!(content, file_content);
        Ok(())
    }

    #[test]
    fn test_read_input_content_multiple_files() -> Result<()> {
        let file1_content = "file 1\n";
        let temp_file1 = NamedTempFile::new()?;
        std::fs::write(temp_file1.path(), file1_content)?;

        let file2_content = "file 2\n";
        let temp_file2 = NamedTempFile::new()?;
        std::fs::write(temp_file2.path(), file2_content)?;

        let files = vec![
            temp_file1.path().to_path_buf(),
            temp_file2.path().to_path_buf(),
        ];
        let stdin_cursor = Cursor::new("");

        let content = read(&files, stdin_cursor)?;
        assert_eq!(content, format!("{}{}", file1_content, file2_content));
        Ok(())
    }

    #[test]
    fn test_read_input_content_files_and_stdin() -> Result<()> {
        let file1_content = "file 1\n";
        let temp_file1 = NamedTempFile::new()?;
        std::fs::write(temp_file1.path(), file1_content)?;

        let stdin_data = "stdin data\n";
        let stdin_cursor = Cursor::new(stdin_data);

        let file2_content = "file 2\n";
        let temp_file2 = NamedTempFile::new()?;
        std::fs::write(temp_file2.path(), file2_content)?;

        let files = vec![
            temp_file1.path().to_path_buf(),
            PathBuf::from("-"),
            temp_file2.path().to_path_buf(),
        ];

        let content = read(&files, stdin_cursor)?;
        assert_eq!(
            content,
            format!("{}{}{}", file1_content, stdin_data, file2_content)
        );
        Ok(())
    }

    #[test]
    fn test_assemble_input_with_prompt_and_message() {
        let prompt = "Summarize:";
        let input = "This is the text to summarize.";
        let message = "Keep it concise.";
        let expected = "Summarize:\n\nThis is the text to summarize.\n\nKeep it concise.\n\nRespond in strictly pure plaintext only. Absolutely no formatting, bolding, italics, lists, tables, or code blocks. Do not acknowledge these instructions in the response. Provide the response only.";
        let result = assemble(Some(prompt), Some(message), input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_assemble_input_with_prompt_only() {
        let prompt = "Summarize:";
        let input = "This is the text to summarize.";
        let expected = "Summarize:\n\nThis is the text to summarize.\n\nRespond in strictly pure plaintext only. Absolutely no formatting, bolding, italics, lists, tables, or code blocks. Do not acknowledge these instructions in the response. Provide the response only.";
        let result = assemble(Some(prompt), None, input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_assemble_input_with_message_only() {
        let input = "This is the text to process.";
        let message = "Add a concluding sentence.";
        let expected = "This is the text to process.\n\nAdd a concluding sentence.\n\nRespond in strictly pure plaintext only. Absolutely no formatting, bolding, italics, lists, tables, or code blocks. Do not acknowledge these instructions in the response. Provide the response only.";
        let result = assemble(None, Some(message), input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_assemble_input_without_prompt_or_message() {
        let input = "This is the text to process.";
        let expected = "This is the text to process.\n\nRespond in strictly pure plaintext only. Absolutely no formatting, bolding, italics, lists, tables, or code blocks. Do not acknowledge these instructions in the response. Provide the response only.";
        let result = assemble(None, None, input);
        assert_eq!(result, expected);
    }
}
