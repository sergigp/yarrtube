use crate::domain::llm_server::{LlmChat, LlmReply, LlmServer, LlmServerUnreachable};
use std::fmt;

/// Where Anthropic's API is and the key to call it with.
#[derive(Clone, Debug)]
#[expect(
    dead_code,
    reason = "walking skeleton: read once the adapter calls the API"
)]
pub struct AnthropicConfig {
    /// e.g. `https://api.anthropic.com`; `YARRTUBE_ANTHROPIC_API_URL`
    /// overrides it in tests.
    pub api_url: String,
    pub api_key: AnthropicApiKey,
}

/// Never printed: `Debug` shows `AnthropicApiKey(***)`.
#[derive(Clone)]
#[expect(
    dead_code,
    reason = "walking skeleton: read once the adapter calls the API"
)]
pub struct AnthropicApiKey(String);

impl AnthropicApiKey {
    #[expect(dead_code, reason = "walking skeleton: built once serve reads the key")]
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Debug for AnthropicApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnthropicApiKey(***)")
    }
}

/// Anthropic's API: `GET /v1/models` (paged), `POST /v1/messages`.
pub struct AnthropicLlmServerRepository {
    #[expect(
        dead_code,
        reason = "walking skeleton: read once the adapter calls the API"
    )]
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
    pub fn chat(&self, _chat: &LlmChat) -> anyhow::Result<LlmReply> {
        Err(anyhow::anyhow!("Anthropic isn't implemented yet").context(LlmServerUnreachable))
    }
}
