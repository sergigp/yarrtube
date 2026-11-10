use std::time::Duration;

/// A summary takes minutes on a small machine, and thinking can loop for an
/// hour: a call runs at least this long before it has failed.
pub const MIN_CHAT_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// One request to an LLM server's model.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmChat {
    pub model: String,
    pub system: String,
    pub user: String,
    pub context_length: u32,
    pub temperature: f64,
    pub thinking: bool,
    pub max_output_tokens: u32,
    /// The JSON schema the answer must follow; `None` for free text.
    pub format: Option<serde_json::Value>,
    /// Past this without an answer, the call has failed.
    pub timeout: Duration,
}
