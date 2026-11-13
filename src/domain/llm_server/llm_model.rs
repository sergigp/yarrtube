/// A model an LLM provider offers: installed on Ollama, or listed by
/// Anthropic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmModel {
    pub name: String,
    /// Anthropic's name for people, e.g. `Claude Haiku 5.5`.
    pub display_name: Option<String>,
    /// Ollama only.
    pub size_bytes: Option<u64>,
    /// Whether the model can reason before answering (Ollama's `thinking`
    /// capability); the server rejects `think` for models without it. Always
    /// `false` for Anthropic, which thinks adaptively.
    pub thinking: bool,
    /// The most input tokens the model takes (Anthropic's
    /// `max_input_tokens`).
    pub context_window: Option<u32>,
}
