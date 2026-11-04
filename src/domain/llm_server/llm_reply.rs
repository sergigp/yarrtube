/// What a model answered to an `LlmChat`, as the server reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmReply {
    pub content: String,
    pub input_tokens: u32,
    pub stopped_at_limit: bool,
    /// Why the server wouldn't answer this request, if it wouldn't.
    pub refusal: Option<String>,
}

impl LlmReply {
    pub fn refused(reason: impl Into<String>) -> Self {
        Self {
            content: String::new(),
            input_tokens: 0,
            stopped_at_limit: false,
            refusal: Some(reason.into()),
        }
    }
}
