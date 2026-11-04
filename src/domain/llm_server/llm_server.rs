use super::LlmModel;

/// An LLM server as it answered a check: its version and installed models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmServer {
    pub version: String,
    pub models: Vec<LlmModel>,
}

impl LlmServer {
    /// The installed model called `name`.
    pub fn model(&self, name: &str) -> Option<&LlmModel> {
        self.models.iter().find(|model| model.name == name)
    }
}
