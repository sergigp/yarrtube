use crate::domain::channel::{Channel, ChannelPreview, ChannelView};
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

#[derive(Debug, Deserialize)]
pub struct UpdateChannelRequest {
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub video_limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PreviewChannelQuery {
    #[serde(default)]
    pub channel: Option<String>,
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

#[derive(Debug, Serialize, PartialEq)]
pub struct ChannelListItemResponse {
    pub id: String,
    pub name: String,
    pub path: String,
    pub quality: String,
    pub video_limit: u32,
    pub avatar_filename: Option<String>,
    pub unwatched_count: usize,
}

impl From<ChannelView> for ChannelListItemResponse {
    fn from(channel: ChannelView) -> Self {
        Self {
            id: channel.id.as_str().to_string(),
            name: channel.name,
            path: channel.path.as_str().to_string(),
            quality: channel.quality.as_str().to_string(),
            video_limit: channel.video_limit.value(),
            avatar_filename: channel.avatar_filename,
            unwatched_count: channel.unwatched_count,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ChannelPreviewResponse {
    pub id: String,
    pub title: String,
    pub avatar_url: Option<String>,
}

impl From<ChannelPreview> for ChannelPreviewResponse {
    fn from(preview: ChannelPreview) -> Self {
        Self {
            id: preview.id.as_str().to_string(),
            title: preview.name,
            avatar_url: preview.avatar_url,
        }
    }
}
