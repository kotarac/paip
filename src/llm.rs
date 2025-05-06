use anyhow::{Result, anyhow};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use std::time::Duration;

use crate::config::Config;

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
}

impl LlmClient {
    pub fn new(provider: LlmProvider, config: &Config) -> Result<Self> {
        let key = config.llm.key.clone();

        if key.is_empty() || key == "YOUR_GEMINI_API_KEY" {
            return Err(anyhow!(
                "API key is not configured for provider: {}",
                provider.as_str()
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(config.llm.timeout))
            .build()?;

        Ok(Self {
            provider,
            api_key: key,
            client,
            config: config.clone(),
        })
    }

    pub fn send_request(&self, prompt: &str) -> Result<String> {
        match self.provider {
            LlmProvider::Gemini => self.send_gemini_request(prompt),
        }
    }

    fn send_gemini_request(&self, prompt: &str) -> Result<String> {
        let gemini_config = self
            .config
            .llm
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
        struct RequestBody {
            contents: Vec<Content>,
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

        let request_body = RequestBody {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
        };

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

        if let Some(ref candidates) = body.candidates {
            if let Some(candidate) = candidates.iter().next() {
                if let Some(ref content) = candidate.content {
                    if let Some(part) = content.parts.iter().next() {
                        return Ok(part.text.clone());
                    }
                }
            }
        }

        Err(anyhow!(
            "LLM response successful but no text content found. Response: {:?}",
            body
        ))
    }
}
