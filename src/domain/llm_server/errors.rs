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
