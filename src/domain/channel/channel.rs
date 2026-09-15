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
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            youtube_channel_id: youtube_channel_id.into(),
            quality,
            video_limit,
            path,
            created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_build_a_channel_from_valid_value_objects() {
        let id = ChannelHandle::new("@somechannel").unwrap();
        let quality = Quality::High;
        let video_limit = VideoLimit::new(10).unwrap();
        let path = PlaylistPath::new("creators/somechannel").unwrap();
        let created_at = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let channel = Channel::create(
            id.clone(),
            "Some Channel",
            "UC123",
            quality,
            video_limit,
            path.clone(),
            created_at,
        );

        assert_eq!(channel.id, id);
        assert_eq!(channel.name, "Some Channel");
        assert_eq!(channel.youtube_channel_id, "UC123");
        assert_eq!(channel.quality, quality);
        assert_eq!(channel.video_limit, video_limit);
        assert_eq!(channel.path, path);
        assert_eq!(channel.created_at, created_at);
    }
}
