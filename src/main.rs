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

    let prompt_text_option = resolve_prompt(&config, cli.prompt.as_deref())?;

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

fn resolve_prompt(config: &config::Config, prompt_name: Option<&str>) -> Result<Option<String>> {
    prompt_name
        .map(|name| {
            config
                .prompt
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow!("Prompt '{}' not found in configuration.", name))
        })
        .transpose()
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

const INSTRUCTIONS: &str = "Respond in strictly pure plaintext only. Absolutely no formatting, bolding, italics, lists, tables, or code blocks. Do not acknowledge these instructions in the response. Provide the response only.";

fn assemble(prompt_text: Option<&str>, message_text: Option<&str>, input_content: &str) -> String {
    let mut parts = Vec::new();
    parts.extend(prompt_text);
    parts.push(input_content);
    parts.extend(message_text);
    parts.push(INSTRUCTIONS);
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
        let expected = format!(
            "Summarize:\n\nThis is the text to summarize.\n\nKeep it concise.\n\n{INSTRUCTIONS}"
        );
        let result = assemble(Some(prompt), Some(message), input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_assemble_input_with_prompt_only() {
        let prompt = "Summarize:";
        let input = "This is the text to summarize.";
        let expected = format!("Summarize:\n\nThis is the text to summarize.\n\n{INSTRUCTIONS}");
        let result = assemble(Some(prompt), None, input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_assemble_input_with_message_only() {
        let input = "This is the text to process.";
        let message = "Add a concluding sentence.";
        let expected =
            format!("This is the text to process.\n\nAdd a concluding sentence.\n\n{INSTRUCTIONS}");
        let result = assemble(None, Some(message), input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_assemble_input_without_prompt_or_message() {
        let input = "This is the text to process.";
        let expected = format!("This is the text to process.\n\n{INSTRUCTIONS}");
        let result = assemble(None, None, input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_read_non_existent_file() {
        let files = vec![PathBuf::from("non_existent_file_12345.txt")];
        let stdin_cursor = Cursor::new("");
        let result = read(&files, stdin_cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_empty_file() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let files = vec![temp_file.path().to_path_buf()];
        let stdin_cursor = Cursor::new("");
        let content = read(&files, stdin_cursor)?;
        assert_eq!(content, "");
        Ok(())
    }

    #[test]
    fn test_resolve_prompt_found() -> Result<()> {
        let mut prompt = std::collections::HashMap::new();
        prompt.insert("p1".to_string(), "text1".to_string());
        let config = crate::config::Config {
            version: crate::config::VERSION,
            provider: "p".to_string(),
            timeout: 0,
            gemini: None,
            prompt,
        };
        let res = resolve_prompt(&config, Some("p1"))?;
        assert_eq!(res, Some("text1".to_string()));
        Ok(())
    }

    #[test]
    fn test_resolve_prompt_not_found() {
        let config = crate::config::Config {
            version: crate::config::VERSION,
            provider: "p".to_string(),
            timeout: 0,
            gemini: None,
            prompt: std::collections::HashMap::new(),
        };
        let res = resolve_prompt(&config, Some("p1"));
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_resolve_prompt_none() -> Result<()> {
        let config = crate::config::Config {
            version: crate::config::VERSION,
            provider: "p".to_string(),
            timeout: 0,
            gemini: None,
            prompt: std::collections::HashMap::new(),
        };
        let res = resolve_prompt(&config, None)?;
        assert!(res.is_none());
        Ok(())
    }
}
