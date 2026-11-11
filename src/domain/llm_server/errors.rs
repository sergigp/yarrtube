use super::OllamaUrl;
use std::fmt;

#[derive(Debug)]
pub enum ReadLlmServerError {
    Unreachable {
        url: OllamaUrl,
        source: anyhow::Error,
    },
}

impl fmt::Display for ReadLlmServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreachable { url, source } => {
                write!(f, "Could not reach Ollama at {url}: {source:#}")
            }
        }
    }
}

impl std::error::Error for ReadLlmServerError {}

/// Ollama couldn't be connected to (refused, unknown host, connect timeout):
/// raised inside `anyhow` by the adapter, checked with `is::<_>()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmServerUnreachable;

impl fmt::Display for LlmServerUnreachable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "can't connect to Ollama")
    }
}

impl std::error::Error for LlmServerUnreachable {}
