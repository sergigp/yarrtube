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
            created_at: channel.created_at,
        }
    }
}
