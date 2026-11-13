use super::OllamaUrl;

/// Which provider a call goes to; the Anthropic key lives in infrastructure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmConnection {
    Ollama(OllamaUrl),
    Anthropic,
}
