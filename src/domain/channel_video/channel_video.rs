use crate::domain::channel::ChannelHandle;
use crate::domain::video::VideoId;
use chrono::{DateTime, Utc};

/// Records that a `Video` belongs to a `Channel`'s tracked most-recent
/// uploads, and its recency rank among them (`0` = most recent). `id` is
/// assigned by storage on insert; `0` before a fresh row has been persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelVideo {
    pub id: i64,
    pub channel_id: ChannelHandle,
    pub video_id: VideoId,
    pub position: i64,
    pub created_at: DateTime<Utc>,
    /// A favorite no longer among the channel's listed uploads, kept anyway.
    pub kept: bool,
}

impl ChannelVideo {
    pub fn create(
        channel_id: ChannelHandle,
        video_id: VideoId,
        position: i64,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: 0,
            channel_id,
            video_id,
            position,
            created_at: now,
            kept: false,
        }
    }

    /// Kept at `position`, after the listed uploads.
    #[allow(dead_code)] // walking skeleton
    pub fn keep(self, _position: i64, _now: DateTime<Utc>) -> Self {
        self
    }
}
