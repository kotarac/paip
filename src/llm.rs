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

        let api_generation_config = self.config.gemini.as_ref().map(|gc| ApiGenerationConfig {
            temperature: gc.temperature,
            top_p: gc.top_p,
            top_k: gc.top_k,
            max_output_tokens: gc.max_output_tokens,
            thinking_config: gc.thinking_budget.map(|tb| ApiThinkingConfig {
                thinking_budget: Some(tb),
            }),
        });

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
