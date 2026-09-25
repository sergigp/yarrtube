use crate::domain::channel::ChannelHandle;
use crate::domain::video::VideoRecordId;
use chrono::{DateTime, Utc};

/// Records that a `Video` belongs to a `Channel`'s tracked most-recent
/// uploads, and its recency rank among them (`0` = most recent). `id` is
/// assigned by storage on insert; `0` before a fresh row has been persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelVideo {
    pub id: i64,
    pub channel_id: ChannelHandle,
    pub video_id: VideoRecordId,
    pub position: i64,
    pub created_at: DateTime<Utc>,
}

impl ChannelVideo {
    pub fn create(
        channel_id: ChannelHandle,
        video_id: VideoRecordId,
        position: i64,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: 0,
            channel_id,
            video_id,
            position,
            created_at: now,
        }
    }
}
