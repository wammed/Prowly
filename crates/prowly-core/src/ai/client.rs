//! BYOK (Bring Your Own Key) AI Client abstraction supporting OpenAI, Anthropic, Gemini, and Ollama.

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error types occurring during AI client requests.
#[derive(Debug, Error)]
pub enum AiError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("API request failed with status {status}: {message}")]
    ApiFailure { status: u16, message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: API key missing for provider '{0}'")]
    MissingApiKey(String),
}

/// Supported AI providers for BYOK configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    #[default]
    #[serde(alias = "openai", alias = "OpenAi", alias = "open-ai")]
    OpenAi,
    #[serde(alias = "anthropic", alias = "Anthropic")]
    Anthropic,
    #[serde(alias = "gemini", alias = "Gemini")]
    Gemini,
    #[serde(alias = "ollama", alias = "Ollama")]
    Ollama,
    #[serde(alias = "custom", alias = "Custom")]
    Custom,
}

impl AiProvider {
    pub const ALL: &'static [AiProvider] = &[
        AiProvider::OpenAi,
        AiProvider::Anthropic,
        AiProvider::Gemini,
        AiProvider::Ollama,
        AiProvider::Custom,
    ];

    pub const fn default_model(&self) -> &'static str {
        match self {
            Self::OpenAi => "gpt-4o-mini",
            Self::Anthropic => "claude-3-5-sonnet-20241022",
            Self::Gemini => "gemini-1.5-flash",
            Self::Ollama => "llama3.2:latest",
            Self::Custom => "default",
        }
    }

    pub const fn default_endpoint(&self) -> &'static str {
        match self {
            Self::OpenAi => "https://api.openai.com/v1",
            Self::Anthropic => "https://api.anthropic.com/v1",
            Self::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            Self::Ollama => "http://localhost:11434/api",
            Self::Custom => "http://localhost:8080/v1",
        }
    }
}

/// Role of a message in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiRole {
    System,
    User,
    Assistant,
}

/// A single message exchanged with an AI model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: AiRole,
    pub content: String,
}

impl AiMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: AiRole::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: AiRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: AiRole::Assistant,
            content: content.into(),
        }
    }
}

/// A request prompt containing conversation history and configuration overrides.
#[derive(Debug, Clone, Default)]
pub struct AiPrompt {
    pub messages: Vec<AiMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl AiPrompt {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(&mut self, message: AiMessage) -> &mut Self {
        self.messages.push(message);
        self
    }

    pub fn system_and_user(system: impl Into<String>, user: impl Into<String>) -> Self {
        Self {
            messages: vec![AiMessage::system(system), AiMessage::user(user)],
            temperature: None,
            max_tokens: None,
        }
    }
}

/// Completed response from an AI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub content: String,
    pub model: String,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
}

/// Configuration settings for an AI client instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: AiProvider,
    pub model: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub num_ctx: Option<u32>,
    /// Ephemeral environment-provided API key (never written to config file).
    #[serde(skip)]
    pub env_api_key: Option<String>,
}

impl AiConfig {
    /// Return the active API key, preferring explicit config key over environment variable key.
    pub fn effective_api_key(&self) -> Option<&str> {
        self.api_key.as_deref().or(self.env_api_key.as_deref())
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::OpenAi,
            model: AiProvider::OpenAi.default_model().to_string(),
            api_key: None,
            api_base: None,
            temperature: Some(0.3),
            max_tokens: Some(2048),
            num_ctx: Some(8192),
            env_api_key: None,
        }
    }
}

/// Unified async interface for all AI backends.
#[async_trait]
pub trait AiClient: Send + Sync {
    async fn complete(&self, prompt: &AiPrompt) -> Result<AiResponse, AiError>;
    fn provider(&self) -> AiProvider;
    fn model(&self) -> &str;
}

/// Create an AI client corresponding to the given configuration.
pub fn create_ai_client(config: &AiConfig) -> Box<dyn AiClient + Send + Sync> {
    match config.provider {
        AiProvider::OpenAi | AiProvider::Custom => Box::new(OpenAiClient::new(config.clone())),
        AiProvider::Anthropic => Box::new(AnthropicClient::new(config.clone())),
        AiProvider::Gemini => Box::new(GeminiClient::new(config.clone())),
        AiProvider::Ollama => Box::new(OllamaClient::new(config.clone())),
    }
}

/// Create a configured reqwest HTTP client with robust connection and request timeouts.
fn default_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Sanitize and truncate HTTP error body to prevent log flooding and credential leakage.
fn sanitize_error_body(body: String) -> String {
    let masked = crate::ai::mask_secrets(&body);
    let trimmed = masked.trim();
    if trimmed.chars().count() > 500 {
        let truncated: String = trimmed.chars().take(500).collect();
        format!("{}... [truncated]", truncated)
    } else {
        trimmed.to_string()
    }
}

// ---------------------------------------------------------------------------
// 1. OpenAI / OpenAI-compatible client
// ---------------------------------------------------------------------------
pub struct OpenAiClient {
    config: AiConfig,
    client: reqwest::Client,
}

impl OpenAiClient {
    pub fn new(config: AiConfig) -> Self {
        Self {
            config,
            client: default_http_client(),
        }
    }
}

#[async_trait]
impl AiClient for OpenAiClient {
    async fn complete(&self, prompt: &AiPrompt) -> Result<AiResponse, AiError> {
        let base = self
            .config
            .api_base
            .as_deref()
            .unwrap_or_else(|| self.config.provider.default_endpoint());
        let url = format!("{}/chat/completions", base.trim_end_matches('/'));

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(key) = self.config.effective_api_key() {
            let auth = format!("Bearer {}", key);
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&auth)
                    .map_err(|e| AiError::Config(format!("Invalid auth header: {}", e)))?,
            );
        } else if self.config.provider == AiProvider::OpenAi {
            return Err(AiError::MissingApiKey("OpenAI".to_string()));
        }

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": prompt.messages,
            "temperature": prompt.temperature.or(self.config.temperature).unwrap_or(0.3),
            "max_tokens": prompt.max_tokens.or(self.config.max_tokens),
        });

        let res = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let raw_msg = res.text().await.unwrap_or_default();
            let message = sanitize_error_body(raw_msg);
            return Err(AiError::ApiFailure { status, message });
        }

        let val: serde_json::Value = res.json().await?;
        let content = val["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let prompt_tokens = val["usage"]["prompt_tokens"].as_u64().map(|v| v as u32);
        let completion_tokens = val["usage"]["completion_tokens"].as_u64().map(|v| v as u32);

        Ok(AiResponse {
            content,
            model: self.config.model.clone(),
            prompt_tokens,
            completion_tokens,
        })
    }

    fn provider(&self) -> AiProvider {
        self.config.provider
    }

    fn model(&self) -> &str {
        &self.config.model
    }
}

// ---------------------------------------------------------------------------
// 2. Anthropic Claude client
// ---------------------------------------------------------------------------
pub struct AnthropicClient {
    config: AiConfig,
    client: reqwest::Client,
}

impl AnthropicClient {
    pub fn new(config: AiConfig) -> Self {
        Self {
            config,
            client: default_http_client(),
        }
    }
}

#[async_trait]
impl AiClient for AnthropicClient {
    async fn complete(&self, prompt: &AiPrompt) -> Result<AiResponse, AiError> {
        let key = self
            .config
            .effective_api_key()
            .ok_or_else(|| AiError::MissingApiKey("Anthropic".to_string()))?;

        let base = self
            .config
            .api_base
            .as_deref()
            .unwrap_or_else(|| self.config.provider.default_endpoint());
        let url = format!("{}/messages", base.trim_end_matches('/'));

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            reqwest::header::HeaderName::from_static("x-api-key"),
            HeaderValue::from_str(key)
                .map_err(|e| AiError::Config(format!("Invalid x-api-key: {}", e)))?,
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("anthropic-version"),
            HeaderValue::from_static("2023-06-01"),
        );

        // Anthropic separates system prompt from conversation messages
        let mut system_text = String::new();
        let mut messages = Vec::new();
        for msg in &prompt.messages {
            match msg.role {
                AiRole::System => {
                    if !system_text.is_empty() {
                        system_text.push('\n');
                    }
                    system_text.push_str(&msg.content);
                }
                AiRole::User | AiRole::Assistant => {
                    messages.push(serde_json::json!({
                        "role": match msg.role {
                            AiRole::User => "user",
                            AiRole::Assistant => "assistant",
                            _ => unreachable!(),
                        },
                        "content": msg.content
                    }));
                }
            }
        }

        let body = serde_json::json!({
            "model": self.config.model,
            "system": if system_text.is_empty() { None } else { Some(system_text) },
            "messages": messages,
            "max_tokens": prompt.max_tokens.or(self.config.max_tokens).unwrap_or(2048),
            "temperature": prompt.temperature.or(self.config.temperature).unwrap_or(0.3),
        });

        let res = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let raw_msg = res.text().await.unwrap_or_default();
            let message = sanitize_error_body(raw_msg);
            return Err(AiError::ApiFailure { status, message });
        }

        let val: serde_json::Value = res.json().await?;
        let content = val["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let prompt_tokens = val["usage"]["input_tokens"].as_u64().map(|v| v as u32);
        let completion_tokens = val["usage"]["output_tokens"].as_u64().map(|v| v as u32);

        Ok(AiResponse {
            content,
            model: self.config.model.clone(),
            prompt_tokens,
            completion_tokens,
        })
    }

    fn provider(&self) -> AiProvider {
        AiProvider::Anthropic
    }

    fn model(&self) -> &str {
        &self.config.model
    }
}

// ---------------------------------------------------------------------------
// 3. Google Gemini client
// ---------------------------------------------------------------------------
pub struct GeminiClient {
    config: AiConfig,
    client: reqwest::Client,
}

impl GeminiClient {
    pub fn new(config: AiConfig) -> Self {
        Self {
            config,
            client: default_http_client(),
        }
    }
}

#[async_trait]
impl AiClient for GeminiClient {
    async fn complete(&self, prompt: &AiPrompt) -> Result<AiResponse, AiError> {
        let key = self
            .config
            .effective_api_key()
            .ok_or_else(|| AiError::MissingApiKey("Gemini".to_string()))?;

        let base = self
            .config
            .api_base
            .as_deref()
            .unwrap_or_else(|| self.config.provider.default_endpoint());
        let url = format!(
            "{}/models/{}:generateContent",
            base.trim_end_matches('/'),
            self.config.model
        );

        let mut contents = Vec::new();
        let mut system_instruction = None;
        for msg in &prompt.messages {
            match msg.role {
                AiRole::System => {
                    system_instruction = Some(serde_json::json!({
                        "parts": [{ "text": msg.content }]
                    }));
                }
                AiRole::User => {
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [{ "text": msg.content }]
                    }));
                }
                AiRole::Assistant => {
                    contents.push(serde_json::json!({
                        "role": "model",
                        "parts": [{ "text": msg.content }]
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "temperature": prompt.temperature.or(self.config.temperature).unwrap_or(0.3),
                "maxOutputTokens": prompt.max_tokens.or(self.config.max_tokens).unwrap_or(2048)
            }
        });
        if let Some(sys) = system_instruction {
            body["systemInstruction"] = sys;
        }

        let res = self
            .client
            .post(&url)
            .header("x-goog-api-key", key)
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let raw_msg = res.text().await.unwrap_or_default();
            let message = sanitize_error_body(raw_msg);
            return Err(AiError::ApiFailure { status, message });
        }

        let val: serde_json::Value = res.json().await?;
        let content = val["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let prompt_tokens = val["usageMetadata"]["promptTokenCount"]
            .as_u64()
            .map(|v| v as u32);
        let completion_tokens = val["usageMetadata"]["candidatesTokenCount"]
            .as_u64()
            .map(|v| v as u32);

        Ok(AiResponse {
            content,
            model: self.config.model.clone(),
            prompt_tokens,
            completion_tokens,
        })
    }

    fn provider(&self) -> AiProvider {
        AiProvider::Gemini
    }

    fn model(&self) -> &str {
        &self.config.model
    }
}

// ---------------------------------------------------------------------------
// 4. Ollama Local LLM client
// ---------------------------------------------------------------------------
pub struct OllamaClient {
    config: AiConfig,
    client: reqwest::Client,
}

impl OllamaClient {
    pub fn new(config: AiConfig) -> Self {
        Self {
            config,
            client: default_http_client(),
        }
    }

    /// Construct the full chat endpoint URL, ensuring `/api/chat` is correctly targeted
    /// even if the user configured `api_base` without the `/api` path suffix.
    pub fn chat_url(&self) -> String {
        let base = self
            .config
            .api_base
            .as_deref()
            .unwrap_or_else(|| self.config.provider.default_endpoint());
        let trimmed = base.trim_end_matches('/');
        if trimmed.ends_with("/api") {
            format!("{}/chat", trimmed)
        } else {
            format!("{}/api/chat", trimmed)
        }
    }
}

#[async_trait]
impl AiClient for OllamaClient {
    async fn complete(&self, prompt: &AiPrompt) -> Result<AiResponse, AiError> {
        let url = self.chat_url();

        let messages: Vec<_> = prompt
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        AiRole::System => "system",
                        AiRole::User => "user",
                        AiRole::Assistant => "assistant",
                    },
                    "content": m.content
                })
            })
            .collect();

        let num_ctx = self.config.num_ctx.unwrap_or(8192);
        let max_tokens = prompt.max_tokens.or(self.config.max_tokens).unwrap_or(2048);

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": prompt.temperature.or(self.config.temperature).unwrap_or(0.3),
                "num_ctx": num_ctx,
                "num_predict": max_tokens,
            }
        });

        let res = self.client.post(&url).json(&body).send().await?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let raw_msg = res.text().await.unwrap_or_default();
            let message = sanitize_error_body(raw_msg);
            return Err(AiError::ApiFailure { status, message });
        }

        let val: serde_json::Value = res.json().await?;
        let content = val["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let prompt_tokens = val["prompt_eval_count"].as_u64().map(|v| v as u32);
        let completion_tokens = val["eval_count"].as_u64().map(|v| v as u32);

        Ok(AiResponse {
            content,
            model: self.config.model.clone(),
            prompt_tokens,
            completion_tokens,
        })
    }

    fn provider(&self) -> AiProvider {
        AiProvider::Ollama
    }

    fn model(&self) -> &str {
        &self.config.model
    }
}

// ---------------------------------------------------------------------------
// 5. Mock Client for Unit Tests
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct MockAiClient {
    pub response: String,
    pub model: String,
}

impl MockAiClient {
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
            model: "mock-model".to_string(),
        }
    }
}

#[async_trait]
impl AiClient for MockAiClient {
    async fn complete(&self, _prompt: &AiPrompt) -> Result<AiResponse, AiError> {
        Ok(AiResponse {
            content: self.response.clone(),
            model: self.model.clone(),
            prompt_tokens: Some(10),
            completion_tokens: Some(25),
        })
    }

    fn provider(&self) -> AiProvider {
        AiProvider::Custom
    }

    fn model(&self) -> &str {
        &self.model
    }
}
