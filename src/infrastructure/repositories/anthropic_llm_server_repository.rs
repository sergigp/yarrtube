use crate::domain::llm_server::{LlmChat, LlmReply, LlmServer, LlmServerUnreachable};
use std::fmt;
use std::time::Duration;

const ANTHROPIC_VERSION: &str = "2023-06-01";
/// How long connecting may take before Anthropic counts as unreachable.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Where Anthropic's API is and the key to call it with.
#[derive(Clone, Debug)]
pub struct AnthropicConfig {
    /// e.g. `https://api.anthropic.com`; `YARRTUBE_ANTHROPIC_API_URL`
    /// overrides it in tests.
    pub api_url: String,
    pub api_key: AnthropicApiKey,
}

/// Never printed: `Debug` shows `AnthropicApiKey(***)`.
#[derive(Clone)]
pub struct AnthropicApiKey(String);

impl AnthropicApiKey {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AnthropicApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnthropicApiKey(***)")
    }
}

/// Anthropic's API: `GET /v1/models` (paged), `POST /v1/messages`.
pub struct AnthropicLlmServerRepository {
    config: AnthropicConfig,
}

impl AnthropicLlmServerRepository {
    pub fn new(config: AnthropicConfig) -> Self {
        Self { config }
    }

    /// The models the key can use, as Anthropic lists them, newest first.
    pub fn find(&self) -> anyhow::Result<LlmServer> {
        Err(anyhow::anyhow!("Anthropic isn't implemented yet").context(LlmServerUnreachable))
    }

    /// What the model says to `chat`: `POST /v1/messages`. Errors tagged
    /// `LlmServerUnreachable`: connect, 401, 403, 404, 400 credit. Plain
    /// errors: 429, 5xx, 529. `LlmReply::refused*`: other 400s, refusal.
    pub fn chat(&self, chat: &LlmChat) -> anyhow::Result<LlmReply> {
        let client = Self::client(chat.timeout)?;
        client
            .post(format!("{}/v1/messages", self.config.api_url))
            .header("x-api-key", self.config.api_key.as_str())
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&Self::message_body(chat))
            .send()?;
        Ok(LlmReply::refused(""))
    }

    /// The request body: no temperature, context length or thinking, so the
    /// model's default adaptive thinking applies.
    /// The answer's schema, when the chat has one, goes as structured output.
    fn message_body(chat: &LlmChat) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": chat.model,
            "max_tokens": chat.max_output_tokens,
            "system": chat.system,
            "messages": [{"role": "user", "content": chat.user}],
            "output_config": {"effort": chat.effort.as_str()},
        });
        if let Some(schema) = &chat.format {
            body["output_config"]["format"] = serde_json::json!({
                "type": "json_schema",
                "schema": Self::closed(schema),
            });
        }
        body
    }

    /// `schema` with `additionalProperties: false` on every object, which
    /// structured output requires.
    fn closed(schema: &serde_json::Value) -> serde_json::Value {
        match schema {
            serde_json::Value::Object(fields) => {
                let mut closed: serde_json::Map<_, _> = fields
                    .iter()
                    .map(|(key, value)| (key.clone(), Self::closed(value)))
                    .collect();
                if closed.get("type").and_then(serde_json::Value::as_str) == Some("object") {
                    closed
                        .entry("additionalProperties")
                        .or_insert(serde_json::Value::Bool(false));
                }
                serde_json::Value::Object(closed)
            }
            serde_json::Value::Array(items) => items.iter().map(Self::closed).collect(),
            other => other.clone(),
        }
    }

    /// A fresh client per call, as for Ollama.
    fn client(timeout: Duration) -> anyhow::Result<reqwest::blocking::Client> {
        Ok(reqwest::blocking::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(timeout)
            .build()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm_server::{Effort, MIN_CHAT_TIMEOUT};

    #[test]
    fn it_should_send_the_api_key_and_version_headers() {
        let mut server = mockito::Server::new();
        let messages = server
            .mock("POST", "/v1/messages")
            .match_header("x-api-key", "sk-ant-test")
            .match_header("anthropic-version", "2023-06-01")
            .with_body(ANSWER)
            .create();

        let result = repository(&server).chat(&chat());

        assert_eq!(result.map(|_| ()).map_err(|e| e.to_string()), Ok(()));
        messages.assert();
    }

    #[test]
    fn it_should_post_the_message_with_model_max_tokens_and_effort() {
        let mut server = mockito::Server::new();
        let messages = server
            .mock("POST", "/v1/messages")
            .match_body(mockito::Matcher::Json(serde_json::json!({
                "model": "claude-haiku-5-5",
                "max_tokens": 16000,
                "system": "Be brief.",
                "messages": [{"role": "user", "content": "Title: Sicilian"}],
                "output_config": {"effort": "medium"},
            })))
            .with_body(ANSWER)
            .create();

        let result = repository(&server).chat(&chat());

        assert_eq!(result.map(|_| ()).map_err(|e| e.to_string()), Ok(()));
        messages.assert();
    }

    #[test]
    fn it_should_send_the_schema_as_structured_output() {
        let mut server = mockito::Server::new();
        let schema = serde_json::json!({
            "type": "object",
            "properties": {"Tech": {
                "type": "object",
                "properties": {"reason": {"type": "string"}, "belongs": {"type": "boolean"}},
                "required": ["reason", "belongs"],
            }},
            "required": ["Tech"],
        });
        let messages = server
            .mock("POST", "/v1/messages")
            .match_body(mockito::Matcher::PartialJson(serde_json::json!({
                "output_config": {
                    "effort": "medium",
                    "format": {"type": "json_schema", "schema": {
                        "type": "object",
                        "properties": {"Tech": {
                            "type": "object",
                            "properties": {
                                "reason": {"type": "string"},
                                "belongs": {"type": "boolean"},
                            },
                            "required": ["reason", "belongs"],
                            "additionalProperties": false,
                        }},
                        "required": ["Tech"],
                        "additionalProperties": false,
                    }},
                },
            })))
            .with_body(ANSWER)
            .create();
        let with_schema = LlmChat {
            format: Some(schema),
            ..chat()
        };

        let result = repository(&server).chat(&with_schema);

        assert_eq!(result.map(|_| ()).map_err(|e| e.to_string()), Ok(()));
        messages.assert();
    }

    fn repository(server: &mockito::Server) -> AnthropicLlmServerRepository {
        AnthropicLlmServerRepository::new(AnthropicConfig {
            api_url: server.url(),
            api_key: AnthropicApiKey::new("sk-ant-test".to_string()),
        })
    }

    const ANSWER: &str = r#"{"type": "message", "role": "assistant", "content": [{"type": "text", "text": "Three games."}], "stop_reason": "end_turn", "usage": {"input_tokens": 900, "output_tokens": 120}}"#;

    fn chat() -> LlmChat {
        LlmChat {
            model: "claude-haiku-5-5".to_string(),
            system: "Be brief.".to_string(),
            user: "Title: Sicilian".to_string(),
            context_length: 131_072,
            temperature: 0.3,
            thinking: true,
            effort: Effort::Medium,
            max_output_tokens: 16_000,
            format: None,
            timeout: MIN_CHAT_TIMEOUT,
        }
    }
}
