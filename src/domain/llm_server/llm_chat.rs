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
}
