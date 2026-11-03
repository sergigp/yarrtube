/// A model installed on an LLM server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmModel {
    pub name: String,
    pub size_bytes: u64,
    /// Whether the model can reason before answering (Ollama's `thinking`
    /// capability); the server rejects `think` for models without it.
    pub thinking: bool,
}
