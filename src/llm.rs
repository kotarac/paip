use anyhow::{Result, anyhow};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::{Config, GeminiConfig};

#[derive(Debug, Clone, Copy)]
pub enum LlmProvider {
    Gemini,
}

impl LlmProvider {
    fn as_str(&self) -> &'static str {
        match self {
            LlmProvider::Gemini => "gemini",
        }
    }
}

#[derive(Debug)]
pub struct LlmClient {
    provider: LlmProvider,
    api_key: String,
    client: Client,
    config: Config,
    verbose: bool,
}

impl LlmClient {
    pub fn new(config: &Config, verbose: bool) -> Result<Self> {
        let provider_enum = match config.provider.as_str() {
            "gemini" => LlmProvider::Gemini,
            _ => return Err(anyhow!("Unsupported LLM provider: {}", config.provider)),
        };

        let api_key = match provider_enum {
            LlmProvider::Gemini => config
                .gemini
                .as_ref()
                .ok_or_else(|| anyhow!("Gemini configuration not found for provider 'gemini'"))?
                .key
                .clone(),
        };

        if api_key.is_empty() || api_key == "YOUR_GEMINI_API_KEY" {
            return Err(anyhow!(
                "API key is not configured for provider: {}",
                provider_enum.as_str()
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_millis(config.timeout.into()))
            .build()?;

        Ok(Self {
            provider: provider_enum,
            api_key,
            client,
            config: config.clone(),
            verbose,
        })
    }

    pub fn send_request(&self, prompt: &str) -> Result<String> {
        match self.provider {
            LlmProvider::Gemini => self.send_gemini_request(prompt),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct ApiThinkingConfig {
    #[serde(skip_serializing_if = "Option::is_none", rename = "thinkingBudget")]
    thinking_budget: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "thinkingLevel")]
    thinking_level: Option<String>,
}

#[derive(Serialize)]
struct ApiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none", rename = "temperature")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "topP")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "topK")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxOutputTokens")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "thinkingConfig")]
    thinking_config: Option<ApiThinkingConfig>,
}

impl From<&GeminiConfig> for ApiGenerationConfig {
    fn from(gc: &GeminiConfig) -> Self {
        let thinking_config = if let Some(ref level) = gc.thinking_level {
            Some(ApiThinkingConfig {
                thinking_level: Some(level.clone()),
                thinking_budget: None,
            })
        } else {
            gc.thinking_budget.map(|tb| ApiThinkingConfig {
                thinking_budget: Some(tb),
                thinking_level: None,
            })
        };

        ApiGenerationConfig {
            temperature: gc.temperature,
            top_p: gc.top_p,
            top_k: gc.top_k,
            max_output_tokens: gc.max_output_tokens,
            thinking_config,
        }
    }
}

#[derive(Serialize)]
struct RequestBody {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "generationConfig")]
    generation_config: Option<ApiGenerationConfig>,
}

#[derive(Deserialize, Debug)]
struct ResponseBody {
    candidates: Option<Vec<Candidate>>,
    error: Option<ApiError>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: Option<Content>,
}

#[derive(Deserialize, Debug)]
struct ApiError {
    code: u16,
    message: String,
}

impl LlmClient {
    fn send_gemini_request(&self, prompt: &str) -> Result<String> {
        let gemini_config: &GeminiConfig = self
            .config
            .gemini
            .as_ref()
            .ok_or_else(|| anyhow!("Gemini configuration not found"))?;

        let model = &gemini_config.model;
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, self.api_key
        );

        let api_generation_config = self.config.gemini.as_ref().map(ApiGenerationConfig::from);

        let request_body = RequestBody {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: api_generation_config,
        };

        if self.verbose {
            eprintln!("--- LLM API Request ---");
            eprintln!("URL: {url}");
            eprintln!("Body: {}", serde_json::to_string_pretty(&request_body)?);
            eprintln!("-----------------------");
        }

        let res = self.client.post(&url).json(&request_body).send()?;

        let status = res.status();
        let body_text = res.text()?;

        let body: ResponseBody = serde_json::from_str(&body_text).map_err(|e| {
            anyhow!(
                "Failed to deserialize Gemini API response: {} - Body: {}",
                e,
                body_text
            )
        })?;

        if !status.is_success() {
            if let Some(api_error) = body.error {
                return Err(anyhow!(
                    "LLM API error {}: {}",
                    api_error.code,
                    api_error.message
                ));
            } else {
                return Err(anyhow!(
                    "LLM request failed with status {}: {:?}",
                    status,
                    body
                ));
            }
        }

        if let Some(ref candidates) = body.candidates
            && let Some(candidate) = candidates.iter().next()
            && let Some(ref content) = candidate.content
            && let Some(part) = content.parts.first()
        {
            return Ok(part.text.clone());
        }

        Err(anyhow!(
            "LLM response successful but no text content found. Response: {:?}",
            body
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_new_client_unsupported_provider() {
        let config = Config {
            version: 1,
            provider: "unknown".to_string(),
            timeout: 1000,
            gemini: None,
            prompt: HashMap::new(),
        };
        let result = LlmClient::new(&config, false);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Unsupported LLM provider: unknown"
        );
    }

    #[test]
    fn test_new_client_missing_gemini_config() {
        let config = Config {
            version: 1,
            provider: "gemini".to_string(),
            timeout: 1000,
            gemini: None,
            prompt: HashMap::new(),
        };
        let result = LlmClient::new(&config, false);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Gemini configuration not found for provider 'gemini'"
        );
    }

    #[test]
    fn test_gemini_thinking_config_logic() {
        let mut gc = GeminiConfig {
            key: "key".to_string(),
            model: "model".to_string(),
            temperature: Some(1.0),
            top_p: None,
            top_k: None,
            max_output_tokens: None,
            thinking_budget: Some(100),
            thinking_level: Some("high".to_string()),
        };

        let api_config_high = ApiGenerationConfig::from(&gc);
        let tc_high = api_config_high.thinking_config.unwrap();
        assert_eq!(tc_high.thinking_level, Some("high".to_string()));
        assert!(tc_high.thinking_budget.is_none());

        gc.thinking_level = None;
        let api_config_budget = ApiGenerationConfig::from(&gc);
        let tc_budget = api_config_budget.thinking_config.unwrap();
        assert!(tc_budget.thinking_level.is_none());
        assert_eq!(tc_budget.thinking_budget, Some(100));
    }

    #[test]
    fn test_new_client_default_api_key() {
        let config = Config {
            version: 1,
            provider: "gemini".to_string(),
            timeout: 1000,
            gemini: Some(GeminiConfig {
                key: "YOUR_GEMINI_API_KEY".to_string(),
                model: "model".to_string(),
                temperature: None,
                top_p: None,
                top_k: None,
                max_output_tokens: None,
                thinking_budget: None,
                thinking_level: None,
            }),
            prompt: HashMap::new(),
        };
        let result = LlmClient::new(&config, false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("API key is not configured")
        );
    }
}
