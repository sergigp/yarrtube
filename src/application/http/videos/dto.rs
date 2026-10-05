use crate::domain::video::{HomeVideoView, HomeVideos, VideoSource, VideoView};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct RecordProgressRequest {
    #[serde(default)]
    pub position_seconds: Option<i64>,
    #[serde(default)]
    pub duration_seconds: Option<i64>,
    #[serde(default)]
    pub was_watched: Option<bool>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RecordProgressResponse {
    pub watched: bool,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct VideoResponse {
    pub id: String,
    pub title: String,
    pub status: String,
    pub quality: Option<String>,
    pub filename: Option<String>,
    pub thumbnail_filename: Option<String>,
    pub duration_seconds: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub watched: bool,
    pub position_seconds: i64,
    pub synced_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub channel_name: Option<String>,
}

impl From<VideoView> for VideoResponse {
    fn from(view: VideoView) -> Self {
        let video = view.video;
        let (published_at, description, channel_name) = match view.metadata {
            Some(metadata) => (
                Some(metadata.published_at),
                Some(metadata.plot),
                Some(metadata.studio),
            ),
            None => (None, None, None),
        };
        Self {
            watched: video.is_watched(),
            position_seconds: video.playback_position.seconds(),
            id: video.youtube_id.as_str().to_string(),
            title: video.title,
            status: video.status.as_str().to_string(),
            quality: video.quality.map(|q| q.as_str().to_string()),
            filename: video.filename,
            thumbnail_filename: video.thumbnail_filename,
            duration_seconds: video.duration_seconds,
            created_at: video.created_at,
            updated_at: video.updated_at,
            synced_at: video.synced_at,
            published_at,
            description,
            channel_name,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct HomeVideoSourceResponse {
    pub kind: String,
    pub id: String,
    pub name: String,
    pub path: String,
    pub avatar_filename: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct HomeVideoResponse {
    pub id: String,
    pub title: String,
    pub thumbnail_filename: Option<String>,
    pub duration_seconds: Option<i64>,
    pub watched: bool,
    pub position_seconds: i64,
    pub source: HomeVideoSourceResponse,
}

impl From<HomeVideoView> for HomeVideoResponse {
    fn from(view: HomeVideoView) -> Self {
        let source = match view.source {
            VideoSource::Playlist { id, name, path } => HomeVideoSourceResponse {
                kind: "playlist".to_string(),
                id: id.as_str().to_string(),
                name: name.as_str().to_string(),
                path: path.as_str().to_string(),
                avatar_filename: None,
            },
            VideoSource::Channel {
                handle,
                name,
                path,
                avatar_filename,
            } => HomeVideoSourceResponse {
                kind: "channel".to_string(),
                id: handle.as_str().to_string(),
                name,
                path: path.as_str().to_string(),
                avatar_filename,
            },
        };
        Self {
            id: view.youtube_id.as_str().to_string(),
            title: view.title,
            thumbnail_filename: view.thumbnail_filename,
            duration_seconds: view.duration_seconds,
            watched: view.watched,
            position_seconds: view.position_seconds,
            source,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct HomeResponse {
    pub continue_watching: Vec<HomeVideoResponse>,
    pub quick_watches: Vec<HomeVideoResponse>,
    pub latest: Vec<HomeVideoResponse>,
}

impl From<HomeVideos> for HomeResponse {
    fn from(home: HomeVideos) -> Self {
        Self {
            continue_watching: cards(home.continue_watching),
            quick_watches: cards(home.quick_watches),
            latest: cards(home.latest),
        }
    }
}

fn cards(videos: Vec<HomeVideoView>) -> Vec<HomeVideoResponse> {
    videos.into_iter().map(HomeVideoResponse::from).collect()
}
