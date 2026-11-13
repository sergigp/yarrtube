use super::OllamaUrl;

/// Which provider a call goes to; the Anthropic key lives in infrastructure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmConnection {
    Ollama(OllamaUrl),
    Anthropic,
}

impl LlmConnection {
    /// Why `model` can't be called on this provider: it hasn't it.
    pub fn missing_model(&self, model: &str) -> String {
        match self {
            Self::Ollama(_) => format!("{model} isn't installed on Ollama"),
            Self::Anthropic => format!("{model} isn't listed by Anthropic"),
        }
    }
}
