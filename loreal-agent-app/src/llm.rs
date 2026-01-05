use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub enabled: bool,
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub anthropic_version: String,
}

impl LlmConfig {
    pub fn from_env() -> Option<Self> {
        let enabled = std::env::var("LLM_ENABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if !enabled {
            return None;
        }
        let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "openai".to_string());
        let base_url = std::env::var("LLM_BASE_URL").unwrap_or_else(|_| {
            if provider == "claude" {
                "https://api.anthropic.com".to_string()
            } else {
                "https://api.openai.com/v1".to_string()
            }
        });
        let api_key = std::env::var("LLM_API_KEY").unwrap_or_default();
        let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
        let anthropic_version =
            std::env::var("LLM_ANTHROPIC_VERSION").unwrap_or_else(|_| "2023-06-01".to_string());
        Some(Self {
            enabled,
            provider,
            base_url,
            api_key,
            model,
            anthropic_version,
        })
    }
}

pub struct LlmClient {
    http: reqwest::Client,
    config: LlmConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }

    pub async fn chat_json(&self, messages: &[LlmMessage]) -> Result<Value, String> {
        if self.config.api_key.is_empty() {
            return Err("LLM_API_KEY is required when LLM_ENABLED=1".to_string());
        }
        match self.config.provider.as_str() {
            "openai" | "openai-compatible" => self.call_openai_chat_json(messages).await,
            "claude" => self.call_claude_chat_json(messages).await,
            _ => Err(format!("unsupported provider: {}", self.config.provider)),
        }
    }

    async fn call_openai_chat_json(&self, messages: &[LlmMessage]) -> Result<Value, String> {
        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));
        let body = json!({
            "model": self.config.model,
            "messages": messages,
            "response_format": { "type": "json_object" }
        });
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| err.to_string())?;
        if !response.status().is_success() {
            return Err(format!("llm status {}", response.status()));
        }
        let value = response.json::<Value>().await.map_err(|err| err.to_string())?;
        if std::env::var("LLM_DEBUG")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            eprintln!("LLM raw response: {}", value);
        }
        let content = value
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| "missing content in LLM response".to_string())?;
        let json_str = extract_json_content(content);
        let parsed = serde_json::from_str::<Value>(&json_str).map_err(|err| err.to_string())?;
        Ok(parsed)
    }

    async fn call_claude_chat_json(&self, messages: &[LlmMessage]) -> Result<Value, String> {
        let url = format!("{}/v1/messages", self.config.base_url.trim_end_matches('/'));
        let (system, claude_messages) = split_system_messages(messages);
        let body = json!({
            "model": self.config.model,
            "max_tokens": 64000,
            "system": system,
            "messages": claude_messages,
        });
        let response = self
            .http
            .post(url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", &self.config.anthropic_version)
            .json(&body)
            .send()
            .await
            .map_err(|err| err.to_string())?;
        if !response.status().is_success() {
            return Err(format!("llm status {}", response.status()));
        }
        let value = response.json::<Value>().await.map_err(|err| err.to_string())?;
        if std::env::var("LLM_DEBUG")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            eprintln!("LLM raw response: {}", value);
        }
        let content = value
            .get("content")
            .and_then(|content| content.get(0))
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .ok_or_else(|| "missing content in Claude response".to_string())?;
        let json_str = extract_json_content(content);
        let parsed = serde_json::from_str::<Value>(&json_str).map_err(|err| err.to_string())?;
        Ok(parsed)
    }
}

fn extract_json_content(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.starts_with("```") {
        let mut lines = trimmed.lines();
        let _first = lines.next();
        let mut body = Vec::new();
        for line in lines {
            if line.trim_start().starts_with("```") {
                break;
            }
            body.push(line);
        }
        return body.join("\n").trim().to_string();
    }
    trimmed.to_string()
}

fn split_system_messages(messages: &[LlmMessage]) -> (String, Vec<LlmMessage>) {
    let mut system_parts = Vec::new();
    let mut rest = Vec::new();
    for msg in messages {
        if msg.role == "system" {
            system_parts.push(msg.content.clone());
        } else {
            rest.push(msg.clone());
        }
    }
    (system_parts.join("\n"), rest)
}
