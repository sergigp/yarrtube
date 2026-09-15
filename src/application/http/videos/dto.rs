use crate::domain::video::Video;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct VideoResponse {
    pub id: String,
    pub title: String,
    pub status: String,
    pub quality: Option<String>,
    pub filename: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Video> for VideoResponse {
    fn from(video: Video) -> Self {
        Self {
            id: video.video_id.as_str().to_string(),
            title: video.title,
            status: video.status.as_str().to_string(),
            quality: video.quality.map(|q| q.as_str().to_string()),
            filename: video.filename,
            created_at: video.created_at,
            updated_at: video.updated_at,
        }
    }
}
