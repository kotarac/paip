use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub const VERSION: u32 = 0;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub version: u32,
    pub provider: String,
    pub timeout: u32,
    #[serde(default)]
    pub gemini: Option<GeminiConfig>,
    #[serde(default)]
    pub prompt: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeminiConfig {
    pub key: String,
    pub model: String,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub top_k: Option<u32>,
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
    #[serde(default)]
    pub thinking_budget: Option<u32>,
}

pub fn load() -> Result<Config> {
    let config_path = get_path()?;
    let config_str = fs::read_to_string(&config_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow!(
                "Configuration file not found at {}. Run with --init-config to create a default.",
                config_path.display()
            )
        } else {
            anyhow!(
                "Failed to read configuration file at {}: {}",
                config_path.display(),
                e
            )
        }
    })?;
    let config: Config = toml::from_str(&config_str).map_err(|e| {
        anyhow!(
            "Failed to parse configuration file at {}: {}",
            config_path.display(),
            e
        )
    })?;
    ensure_version(&config)?;
    Ok(config)
}

fn ensure_version(config: &Config) -> Result<()> {
    if config.version != VERSION {
        Err(anyhow!(
            "Configuration file version mismatch. Expected major version {}, found {}. Please update your config file or run with --init-config to generate a new one.",
            VERSION,
            config.version
        ))
    } else {
        Ok(())
    }
}

fn get_path() -> Result<PathBuf> {
    let mut config_dir =
        dirs::config_dir().ok_or_else(|| anyhow!("Could not find config directory"))?;
    config_dir.push("paip");
    config_dir.push("config.toml");
    Ok(config_dir)
}

fn create_default(path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let default_content = r#"version = 0

provider = "gemini"
timeout = 90000

[gemini]
key = "YOUR_GEMINI_API_KEY"
model = "gemini-2.5-flash"
temperature = 0.0
top_p = 0.95
top_k = 64
max_output_tokens = 65536
thinking_budget = 0

[prompt]
summarize = "Summarize the following text."
explain = "Explain the following concept."
french = "Translate the following text to French."
italian = "Translate the following text to Italian."
latin = "Translate the following text to Latin."

commit = """
Write a conventional commit message in the following form.

type(optional scope): description

[optional body]

[optional footer(s)]

Use one of the following types: feat, fix, build, chore, ci, docs, perf, refactor, style, test.

Start the description with a lowercase letter and use the imperative mood.

Write the optional body using complete sentences, proper case, and the imperative mood.

To signify a breaking change, append an ! immediately before the : in the header. A commit with an ! in the header MUST include a BREAKING CHANGE: footer.

Start the footer with BREAKING CHANGE: followed by a colon and a space. After the prefix, explain the breaking change, what it affects, and what migration steps are necessary.
"""

review = """
Please review the following code.

Suggest improvements and explain your reasoning for each suggestion.
"""
"#;
    fs::write(path, default_content)?;
    Ok(())
}

pub fn init_default() -> Result<()> {
    let path = get_path()?;
    create_default(&path)?;
    println!("Default config file created at: {}", path.display());
    println!("Please edit the config file with your LLM provider details.");
    Ok(())
}
