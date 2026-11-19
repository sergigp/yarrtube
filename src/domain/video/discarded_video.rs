use super::youtube_video_id::YoutubeVideoId;
use crate::domain::show::ShowKey;
use chrono::{DateTime, Utc};

/// A Discarded video's YouTube id, remembered by the source it left so the
/// source never adds it back (see the `video-discard` capability).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscardedVideo {
    pub source: ShowKey,
    pub youtube_id: YoutubeVideoId,
    pub created_at: DateTime<Utc>,
}

impl DiscardedVideo {
    pub fn new(source: ShowKey, youtube_id: YoutubeVideoId, now: DateTime<Utc>) -> Self {
        Self {
            source,
            youtube_id,
            created_at: now,
        }
    }
}
