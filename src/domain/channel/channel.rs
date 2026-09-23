use super::channel_handle::ChannelHandle;
use super::video_limit::VideoLimit;
use crate::domain::playlist::PlaylistPath;
use crate::domain::shared::Quality;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub id: ChannelHandle,
    pub name: String,
    pub youtube_channel_id: String,
    pub quality: Quality,
    pub video_limit: VideoLimit,
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
            path,
            avatar_filename,
            created_at,
        }
    }
}
