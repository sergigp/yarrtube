use super::sourced_video::{SourcedVideo, VideoSource};
use super::video_id::VideoId;
use crate::domain::category::CategoryId;
use chrono::{DateTime, Utc};

/// A video card of the home listing: the flat fields a card shows, with the
/// source that tracks the video.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeVideoView {
    pub youtube_id: VideoId,
    pub title: String,
    pub thumbnail_filename: Option<String>,
    pub duration_seconds: Option<i64>,
    pub watched: bool,
    pub position_seconds: i64,
    /// When YouTube published the video, unknown without its metadata.
    pub published_at: Option<DateTime<Utc>>,
    pub source: VideoSource,
    /// Categories having the YouTube video, in category-name order.
    pub category_ids: Vec<CategoryId>,
}

impl From<SourcedVideo> for HomeVideoView {
    fn from(sourced: SourcedVideo) -> Self {
        let video = sourced.video;
        Self {
            watched: video.is_watched(),
            position_seconds: video.playback_position.seconds(),
            youtube_id: video.youtube_id,
            title: video.title,
            thumbnail_filename: video.thumbnail_filename,
            duration_seconds: video.duration_seconds,
            published_at: None,
            source: sourced.source,
            category_ids: Vec::new(),
        }
    }
}

impl HomeVideoView {
    pub fn with_published_at(self, published_at: Option<DateTime<Utc>>) -> Self {
        Self {
            published_at,
            ..self
        }
    }

    pub fn with_category_ids(self, category_ids: Vec<CategoryId>) -> Self {
        Self {
            category_ids,
            ..self
        }
    }
}
