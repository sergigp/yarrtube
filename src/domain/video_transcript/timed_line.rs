/// A spoken line and when it starts, in seconds from the video's start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedLine {
    pub start_seconds: i64,
    pub text: String,
}
