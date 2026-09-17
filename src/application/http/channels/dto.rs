use crate::domain::channel::Channel;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub video_limit: Option<i64>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ChannelResponse {
    pub id: String,
    pub name: String,
    pub youtube_channel_id: String,
    pub quality: String,
    pub video_limit: u32,
    pub path: String,
    pub avatar_filename: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<Channel> for ChannelResponse {
    fn from(channel: Channel) -> Self {
        Self {
            id: channel.id.as_str().to_string(),
            name: channel.name,
            youtube_channel_id: channel.youtube_channel_id,
            quality: channel.quality.as_str().to_string(),
            video_limit: channel.video_limit.value(),
            path: channel.path.as_str().to_string(),
            avatar_filename: channel.avatar_filename,
            created_at: channel.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{ChannelHandle, VideoLimit};
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::shared::Quality;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn channel(avatar_filename: Option<String>) -> Channel {
        Channel::create(
            ChannelHandle::new("@somechannel").unwrap(),
            "Some Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            avatar_filename,
            fixed_timestamp(),
        )
    }

    #[test]
    fn it_should_include_the_avatar_filename_when_present() {
        let response: ChannelResponse = channel(Some("@somechannel.jpg".to_string())).into();

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["avatar_filename"], "@somechannel.jpg");
    }

    #[test]
    fn it_should_omit_the_avatar_filename_when_absent() {
        let response: ChannelResponse = channel(None).into();

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["avatar_filename"], serde_json::Value::Null);
    }
}
