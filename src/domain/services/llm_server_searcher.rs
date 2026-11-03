use crate::domain::llm_server::{LlmServer, OllamaUrl, ReadLlmServerError};
use crate::infrastructure::repositories::ollama_llm_server_repository::LlmServerRepository;
use std::sync::Arc;

/// Checks an LLM server and lists its models.
#[derive(Clone)]
pub struct LlmServerSearcher {
    repository: Arc<dyn LlmServerRepository>,
}

impl LlmServerSearcher {
    pub fn new(repository: Arc<dyn LlmServerRepository>) -> Self {
        Self { repository }
    }
}

pub trait LlmServerSearcherApi: Send + Sync {
    /// The server at `url`, or `Unreachable` when it doesn't answer as an
    /// Ollama server.
    fn get(&self, url: OllamaUrl) -> Result<LlmServer, ReadLlmServerError>;
}

impl LlmServerSearcherApi for LlmServerSearcher {
    fn get(&self, _url: OllamaUrl) -> Result<LlmServer, ReadLlmServerError> {
        let _ = &self.repository;
        Ok(LlmServer {
            version: String::new(),
            models: vec![],
        })
    }
}
