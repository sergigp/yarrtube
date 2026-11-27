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
    pub fn keep(self, position: i64, now: DateTime<Utc>) -> Self {
        Self {
            position,
            kept: true,
            created_at: now,
            ..self
        }
    }

    /// Listed again at `position`, no longer kept.
    pub fn listed_at(self, position: i64, now: DateTime<Utc>) -> Self {
        Self {
            position,
            created_at: now,
            kept: false,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_keep_a_channel_video_at_a_new_position() {
        let created_at = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let channel_video = ChannelVideo {
            id: 7,
            ..ChannelVideo::create(
                ChannelHandle::new("@channel").unwrap(),
                VideoId::new_generated(),
                2,
                created_at,
            )
        };

        let kept = channel_video.clone().keep(5, now);

        assert_eq!(
            kept,
            ChannelVideo {
                position: 5,
                created_at: now,
                kept: true,
                ..channel_video
            }
        );
    }
}
