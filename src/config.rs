use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub const VERSION: u32 = 1;

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
    #[serde(default)]
    pub thinking_level: Option<String>,
}

pub fn load() -> Result<Config> {
    let config_path = get_path()?;
    let config_str = fs::read_to_string(&config_path).with_context(|| {
        format!(
            "Failed to read configuration file at {}. Run with --init-config to create a default.",
            config_path.display()
        )
    })?;
    let config: Config = toml::from_str(&config_str).with_context(|| {
        format!(
            "Failed to parse configuration file at {}",
            config_path.display()
        )
    })?;
    ensure_version(&config)?;
    Ok(config)
}

fn ensure_version(config: &Config) -> Result<()> {
    anyhow::ensure!(
        config.version == VERSION,
        "Configuration file version mismatch. Expected major version {}, found {}. Please update your config file or run with --init-config to generate a new one.",
        VERSION,
        config.version
    );
    Ok(())
}

fn get_path() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|d| d.join("paip").join("config.toml"))
        .ok_or_else(|| anyhow!("Could not find config directory"))
}

fn create_default(path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let default_content = include_str!("../config.toml");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_version_match() {
        let config = Config {
            version: VERSION,
            provider: "gemini".to_string(),
            timeout: 1000,
            gemini: None,
            prompt: HashMap::new(),
        };
        assert!(ensure_version(&config).is_ok());
    }

    #[test]
    fn test_default_config_is_valid() {
        let default_content = include_str!("../config.toml");
        let config: Result<Config, _> = toml::from_str(default_content);
        assert!(
            config.is_ok(),
            "Default config should be valid TOML: {:?}",
            config.err()
        );
        let config = config.unwrap();
        assert_eq!(config.version, VERSION);
    }

    #[test]
    fn test_ensure_version_mismatch() {
        let config = Config {
            version: VERSION + 1,
            provider: "gemini".to_string(),
            timeout: 1000,
            gemini: None,
            prompt: HashMap::new(),
        };
        assert!(ensure_version(&config).is_err());
    }
}
