/// How much of the thinking a limit-stop log line keeps, in characters.
const THINKING_TAIL_CHARS: usize = 300;

/// What a model answered to an `LlmChat`, as the server reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmReply {
    pub content: String,
    /// What the model thought before answering; empty without thinking.
    pub thinking: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub stopped_at_limit: bool,
    /// Why the server wouldn't answer this request, if it wouldn't.
    pub refusal: Option<String>,
}

impl LlmReply {
    pub fn refused(reason: impl Into<String>) -> Self {
        Self {
            content: String::new(),
            thinking: String::new(),
            input_tokens: 0,
            output_tokens: 0,
            stopped_at_limit: false,
            refusal: Some(reason.into()),
        }
    }

    /// The last `THINKING_TAIL_CHARS` characters of the thinking.
    pub fn thinking_tail(&self) -> &str {
        let start = self
            .thinking
            .char_indices()
            .rev()
            .nth(THINKING_TAIL_CHARS - 1)
            .map_or(0, |(index, _)| index);
        &self.thinking[start..]
    }
}
