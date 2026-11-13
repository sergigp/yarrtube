use super::LlmConnection;
use std::fmt;

#[derive(Debug)]
pub enum ReadLlmServerError {
    /// Anthropic was asked for without a key.
    NotConfigured,
    Unreachable {
        connection: LlmConnection,
        source: anyhow::Error,
    },
}

impl fmt::Display for ReadLlmServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfigured => write!(
                f,
                "Anthropic isn't configured: set YARRTUBE_ANTHROPIC_API_KEY"
            ),
            Self::Unreachable {
                connection: LlmConnection::Ollama(url),
                source,
            } => write!(f, "Could not reach Ollama at {url}: {source:#}"),
            Self::Unreachable {
                connection: LlmConnection::Anthropic,
                source,
            } => write!(f, "Could not reach Anthropic: {source:#}"),
        }
    }
}

impl std::error::Error for ReadLlmServerError {}

/// The LLM provider is unavailable (Ollama can't be connected to; Anthropic
/// can't be connected to, has no key, rejects it or is out of credit):
/// raised inside `anyhow` by the adapter, checked with `is::<_>()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmServerUnreachable;

impl fmt::Display for LlmServerUnreachable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the LLM provider is unavailable")
    }
}

impl std::error::Error for LlmServerUnreachable {}
