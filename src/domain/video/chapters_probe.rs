use super::video_chapter::VideoChapter;
use chrono::{DateTime, Utc};

/// A video's chapters (`vec![]` for none) and its publish time, when known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChaptersProbe {
    pub chapters: Vec<VideoChapter>,
    pub published_at: Option<DateTime<Utc>>,
}
