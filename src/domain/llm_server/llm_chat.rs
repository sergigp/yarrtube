use super::Effort;
use std::time::Duration;

/// A summary takes minutes on a small machine, and thinking can loop for an
/// hour: a call runs at least this long before it has failed.
pub const MIN_CHAT_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// One request to an LLM provider's model.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmChat {
    pub model: String,
    pub system: String,
    pub user: String,
    /// Ollama only.
    pub context_length: u32,
    /// Ollama only.
    pub temperature: f64,
    /// Ollama only.
    pub thinking: bool,
    /// Anthropic only.
    pub effort: Effort,
    pub max_output_tokens: u32,
    /// The JSON schema the answer must follow; `None` for free text.
    pub format: Option<serde_json::Value>,
    /// Past this without an answer, the call has failed.
    pub timeout: Duration,
}
