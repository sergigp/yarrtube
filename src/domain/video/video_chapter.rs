use serde::{Deserialize, Serialize};

/// One of a video's YouTube chapters: where it starts and ends, in whole
/// seconds from the start of the video, and its title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoChapter {
    pub start_seconds: i64,
    pub end_seconds: i64,
    pub title: String,
}

impl VideoChapter {
    /// `<start> <title>`, as a chapter is listed to a model.
    pub fn listed(&self) -> String {
        format!("{} {}", clock(self.start_seconds), self.title)
    }
}

/// `seconds` as a viewer reads it: `2:05`, or `1:02:05` from an hour on.
pub fn clock(seconds: i64) -> String {
    match seconds >= 3600 {
        true => format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds % 3600 / 60,
            seconds % 60
        ),
        false => format!("{}:{:02}", seconds / 60, seconds % 60),
    }
}
