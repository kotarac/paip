use anyhow::{Result, anyhow};
use dirs;
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

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::get_default_config_path()?;
        let config_str = fs::read_to_string(&config_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow!("Configuration file not found at {}. Run with --init-config to create a default.", config_path.display())
            } else {
                anyhow!("Failed to read configuration file at {}: {}", config_path.display(), e)
            }
        })?;
        let config: Self = serde_yaml::from_str(&config_str).map_err(|e| {
            anyhow!(
                "Failed to parse configuration file at {}: {}",
                config_path.display(),
                e
            )
        })?;
        config.check_version(VERSION)?;
        Ok(config)
    }

    pub fn check_version(&self, current_major_version: u32) -> Result<()> {
        if self.version != current_major_version {
            Err(anyhow!(
                "Configuration file version mismatch. Expected major version {}, found {}. Please update your config file or run with --init-config to generate a new one.",
                current_major_version,
                self.version
            ))
        } else {
            Ok(())
        }
    }

    pub fn get_default_config_path() -> Result<PathBuf> {
        let mut config_dir =
            dirs::config_dir().ok_or_else(|| anyhow!("Could not find config directory"))?;
        config_dir.push("paip");
        config_dir.push("config.yaml");
        Ok(config_dir)
    }

    pub fn create_default_config(path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let default_content = r#"version: 0

provider: gemini
timeout: 30000

gemini:
  key: YOUR_GEMINI_API_KEY
  model: gemini-2.5-flash-preview-04-17
  temperature: 0
  top_p: 0.95
  top_k: 40
  max_output_tokens: 8192
  thinking_budget: 0

prompt:
  summarize: "Summarize the following text."
  explain: "Explain the following concept."
  french: "Translate the following text to French."
"#;
        fs::write(path, default_content)?;
        Ok(())
    }

    pub fn get_prompt(&self, name: &str) -> Result<String> {
        self.prompt
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("Prompt '{}' not found in configuration.", name))
    }
}
