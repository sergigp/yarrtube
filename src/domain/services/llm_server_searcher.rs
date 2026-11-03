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
    /// The server at `url` with its models sorted by name, or `Unreachable`
    /// when it doesn't answer as an Ollama server.
    fn get(&self, url: OllamaUrl) -> Result<LlmServer, ReadLlmServerError>;
}

impl LlmServerSearcherApi for LlmServerSearcher {
    fn get(&self, url: OllamaUrl) -> Result<LlmServer, ReadLlmServerError> {
        let server = self
            .repository
            .find(&url)
            .map_err(|source| ReadLlmServerError::Unreachable { url, source })?;
        Ok(Self::sorted_by_name(server))
    }
}

impl LlmServerSearcher {
    fn sorted_by_name(server: LlmServer) -> LlmServer {
        let mut models = server.models;
        models.sort_by(|a, b| a.name.cmp(&b.name));
        LlmServer { models, ..server }
    }
}
