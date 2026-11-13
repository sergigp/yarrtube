use crate::domain::llm_server::{LlmConnection, LlmServer, ReadLlmServerError};
use crate::infrastructure::repositories::llm_server_repository::LlmServerRepository;
use std::sync::Arc;

/// Checks an LLM provider and lists its models.
#[derive(Clone)]
pub struct LlmServerSearcher {
    repository: Arc<dyn LlmServerRepository>,
    anthropic_available: bool,
}

impl LlmServerSearcher {
    pub fn new(repository: Arc<dyn LlmServerRepository>, anthropic_available: bool) -> Self {
        Self {
            repository,
            anthropic_available,
        }
    }
}

pub trait LlmServerSearcherApi: Send + Sync {
    /// The provider behind `connection` with its models: Ollama's sorted by
    /// name, Anthropic's as listed, newest first. `NotConfigured` for
    /// Anthropic without a key, `Unreachable` when it doesn't answer as the
    /// provider does.
    fn get(&self, connection: LlmConnection) -> Result<LlmServer, ReadLlmServerError>;
}

impl LlmServerSearcherApi for LlmServerSearcher {
    fn get(&self, connection: LlmConnection) -> Result<LlmServer, ReadLlmServerError> {
        if connection == LlmConnection::Anthropic && !self.anthropic_available {
            return Err(ReadLlmServerError::NotConfigured);
        }
        let server = self
            .repository
            .find(&connection)
            .map_err(|source| ReadLlmServerError::Unreachable { connection, source })?;
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
