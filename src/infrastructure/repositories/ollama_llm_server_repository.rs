use crate::domain::llm_server::{LlmServer, OllamaUrl};
use std::time::Duration;

/// A check is interactive (the "Test connection" button), so a server that
/// doesn't answer fails it quickly.
#[allow(dead_code)]
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Port to an LLM server's HTTP API.
pub trait LlmServerRepository: Send + Sync {
    /// The server at `url`: its version and installed models, sorted by
    /// name, each with whether it can think.
    fn find(&self, url: &OllamaUrl) -> anyhow::Result<LlmServer>;
}

/// The Ollama API: `GET /api/version`, `GET /api/tags`, `POST /api/show`.
pub struct OllamaLlmServerRepository;

impl OllamaLlmServerRepository {
    pub fn new() -> Self {
        Self
    }
}

impl LlmServerRepository for OllamaLlmServerRepository {
    fn find(&self, _url: &OllamaUrl) -> anyhow::Result<LlmServer> {
        Ok(LlmServer {
            version: String::new(),
            models: vec![],
        })
    }
}

/// An LLM server answering with a fixed state, or unreachable.
#[cfg(test)]
pub struct FakeLlmServerRepository {
    server: Option<LlmServer>,
}

#[cfg(test)]
impl FakeLlmServerRepository {
    pub fn new(server: LlmServer) -> Self {
        Self {
            server: Some(server),
        }
    }

    /// A server that is down: every call fails.
    pub fn failing() -> Self {
        Self { server: None }
    }
}

#[cfg(test)]
impl LlmServerRepository for FakeLlmServerRepository {
    fn find(&self, _url: &OllamaUrl) -> anyhow::Result<LlmServer> {
        self.server
            .clone()
            .ok_or_else(|| anyhow::anyhow!("connection refused"))
    }
}
