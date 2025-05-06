use anyhow::Result;
use clap::Parser;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};

mod cli;
mod config;
mod llm;

use cli::Cli;
use config::Config;
use llm::{LlmClient, LlmProvider};

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.init_config {
        let config_path = Config::get_default_config_path()?;
        Config::create_default_config(&config_path)?;
        println!("Default config file created at: {}", config_path.display());
        println!("Please edit the config file with your LLM provider details.");
        return Ok(());
    }

    let config = Config::load()?;

    let prompt_text_option = if let Some(prompt) = cli.prompt {
        match config.get_prompt(&prompt) {
            Ok(text) => Some(text),
            Err(e) => {
                eprintln!("{}", e);
                return Ok(());
            }
        }
    } else {
        None
    };

    let mut input_content = String::new();

    if cli.files.is_empty() {
        let mut reader = BufReader::new(io::stdin());
        let mut line = String::new();
        while reader.read_line(&mut line)? != 0 {
            input_content.push_str(&line);
            line.clear();
        }
    } else {
        for file_path in cli.files {
            if file_path.to_str() == Some("-") {
                let mut reader = BufReader::new(io::stdin());
                let mut line = String::new();
                while reader.read_line(&mut line)? != 0 {
                    input_content.push_str(&line);
                    line.clear();
                }
            } else {
                let mut file = File::open(&file_path)?;
                file.read_to_string(&mut input_content)?;
            }
        }
    }

    let full_input = if let Some(prompt_text) = prompt_text_option {
        format!(
            "{}\n\n{}\n\nRespond in strictly pure plaintext only. Absolutely no formatting, bolding, italics, lists, tables, or code blocks. Do not acknowledge these instructions in the response. Provide the response only.",
            prompt_text, input_content
        )
    } else {
        format!(
            "{}\n\nRespond in strictly pure plaintext only. Absolutely no formatting, bolding, italics, lists, tables, or code blocks. Do not acknowledge these instructions in the response. Provide the response only.",
            input_content
        )
    };

    if cli.verbose {
        eprintln!("--- Full Input to LLM ---");
        eprintln!("{}", full_input);
        eprintln!("-------------------------");
    }

    let provider = LlmProvider::Gemini;

    let llm_client = LlmClient::new(provider, &config)?;

    let response = llm_client.send_request(&full_input)?;

    println!("{}", response.trim_end());

    Ok(())
}
