use super::video_status::VideoStatus;
use crate::domain::shared::{PlaylistId, VideoId};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    pub playlist_id: PlaylistId,
    pub video_id: VideoId,
    pub title: String,
    pub status: VideoStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Video {
    pub fn create(
        playlist_id: PlaylistId,
        video_id: VideoId,
        title: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            playlist_id,
            video_id,
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
            PlaylistId::new("PL1").unwrap(),
            VideoId::new("vid1").unwrap(),
            "My Video",
            now,
        );

        assert_eq!(video.status, VideoStatus::Pending);
        assert_eq!(video.title, "My Video");
        assert_eq!(video.created_at, now);
        assert_eq!(video.updated_at, now);
    }
}
