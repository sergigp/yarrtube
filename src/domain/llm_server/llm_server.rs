use super::LlmModel;

/// An LLM provider as it answered a check: its version (Ollama only) and
/// models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmServer {
    pub version: Option<String>,
    pub models: Vec<LlmModel>,
}

impl LlmServer {
    /// The model called `name`.
    pub fn model(&self, name: &str) -> Option<&LlmModel> {
        self.models.iter().find(|model| model.name == name)
    }
}
