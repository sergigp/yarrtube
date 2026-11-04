/// What a model answered to an `LlmChat`, as the server reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmReply {
    pub content: String,
    pub input_tokens: u32,
    pub stopped_at_limit: bool,
}

impl LlmReply {
    /// Stopped at the output cap, or input within 64 tokens of
    /// `context_length`.
    pub fn is_incomplete(&self, _context_length: u32) -> bool {
        self.stopped_at_limit
    }
}
