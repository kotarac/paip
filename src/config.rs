use anyhow::{Result, anyhow};
use dirs;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub llm: LlmConfig,
    #[serde(default)]
    pub prompt: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    pub key: String,
    #[serde(default)]
    pub gemini: Option<GeminiConfig>,
    #[serde(default = "default_timeout_value")]
    pub timeout: u64,
}

fn default_timeout_value() -> u64 {
    60
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeminiConfig {
    pub model: String,
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
        Ok(config)
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
        let default_content = r#"llm:
  provider: gemini
  key: YOUR_GEMINI_API_KEY
  timeout: 30
  gemini:
    model: gemini-2.0-flash

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
