/// One consecutive part of a transcript, in seconds from the video's start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptPart {
    pub start_seconds: i64,
    pub end_seconds: i64,
    pub text: String,
}
