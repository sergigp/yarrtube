use crate::domain::video::{RecentVideo, Video, VideoSource};
use chrono::{DateTime, Utc};
use serde::Serialize;

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
}

impl From<Video> for VideoResponse {
    fn from(video: Video) -> Self {
        Self {
            id: video.youtube_id.as_str().to_string(),
            title: video.title,
            status: video.status.as_str().to_string(),
            quality: video.quality.map(|q| q.as_str().to_string()),
            filename: video.filename,
            thumbnail_filename: video.thumbnail_filename,
            duration_seconds: video.duration_seconds,
            created_at: video.created_at,
            updated_at: video.updated_at,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RecentVideoSourceResponse {
    pub kind: String,
    pub id: String,
    pub path: String,
    pub avatar_filename: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RecentVideoResponse {
    pub id: String,
    pub title: String,
    pub thumbnail_filename: Option<String>,
    pub duration_seconds: Option<i64>,
    pub source: RecentVideoSourceResponse,
}

impl From<RecentVideo> for RecentVideoResponse {
    fn from(recent_video: RecentVideo) -> Self {
        let source = match recent_video.source {
            VideoSource::Playlist(id, path) => RecentVideoSourceResponse {
                kind: "playlist".to_string(),
                id: id.as_str().to_string(),
                path: path.as_str().to_string(),
                avatar_filename: None,
            },
            VideoSource::Channel(id, path, avatar_filename) => RecentVideoSourceResponse {
                kind: "channel".to_string(),
                id: id.as_str().to_string(),
                path: path.as_str().to_string(),
                avatar_filename,
            },
        };
        Self {
            id: recent_video.video.youtube_id.as_str().to_string(),
            title: recent_video.video.title,
            thumbnail_filename: recent_video.video.thumbnail_filename,
            duration_seconds: recent_video.video.duration_seconds,
            source,
        }
    }
}
