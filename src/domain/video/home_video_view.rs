use super::sourced_video::{SourcedVideo, VideoSource};
use super::video_id::VideoId;

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
    pub source: VideoSource,
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
            source: sourced.source,
        }
    }
}
