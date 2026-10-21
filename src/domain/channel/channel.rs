use super::channel_handle::ChannelHandle;
use super::video_limit::VideoLimit;
use crate::domain::playlist::PlaylistPath;
use crate::domain::shared::{Quality, ReconcileInterval};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub id: ChannelHandle,
    pub name: String,
    pub youtube_channel_id: String,
    pub quality: Quality,
    pub video_limit: VideoLimit,
    pub reconcile_interval: ReconcileInterval,
    pub path: PlaylistPath,
    pub avatar_filename: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Channel {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        id: ChannelHandle,
        name: impl Into<String>,
        youtube_channel_id: impl Into<String>,
        quality: Quality,
        video_limit: VideoLimit,
        reconcile_interval: ReconcileInterval,
        path: PlaylistPath,
        avatar_filename: Option<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            youtube_channel_id: youtube_channel_id.into(),
            quality,
            video_limit,
            reconcile_interval,
            path,
            avatar_filename,
            created_at,
        }
    }

    pub fn with_path(self, path: PlaylistPath) -> Self {
        Self { path, ..self }
    }

    pub fn with_quality(self, quality: Quality) -> Self {
        Self { quality, ..self }
    }

    pub fn with_video_limit(self, video_limit: VideoLimit) -> Self {
        Self {
            video_limit,
            ..self
        }
    }

    pub fn with_reconcile_interval(self, reconcile_interval: ReconcileInterval) -> Self {
        Self {
            reconcile_interval,
            ..self
        }
    }

    pub fn with_avatar_filename(self, avatar_filename: Option<String>) -> Self {
        Self {
            avatar_filename,
            ..self
        }
    }
}
