use serde::{Deserialize, Serialize};

/// One of a video's YouTube chapters: where it starts and ends, in whole
/// seconds from the start of the video, and its title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoChapter {
    pub start_seconds: i64,
    pub end_seconds: i64,
    pub title: String,
}
