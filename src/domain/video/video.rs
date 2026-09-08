use super::video_status::VideoStatus;
use super::youtube_video_id::YoutubeVideoId;
use crate::domain::playlist::YoutubePlaylistId;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    pub playlist_id: YoutubePlaylistId,
    pub youtube_video_id: YoutubeVideoId,
    pub title: String,
    pub status: VideoStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Video {
    pub fn create(
        playlist_id: YoutubePlaylistId,
        youtube_video_id: YoutubeVideoId,
        title: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            playlist_id,
            youtube_video_id,
            title: title.into(),
            status: VideoStatus::Pending,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_build_a_pending_video() {
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let video = Video::create(
            YoutubePlaylistId::new("PL1").unwrap(),
            YoutubeVideoId::new("vid1").unwrap(),
            "My Video",
            now,
        );

        assert_eq!(video.status, VideoStatus::Pending);
        assert_eq!(video.title, "My Video");
        assert_eq!(video.created_at, now);
        assert_eq!(video.updated_at, now);
    }
}
