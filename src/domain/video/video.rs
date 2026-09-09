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

    pub fn start_download(self, now: DateTime<Utc>) -> Self {
        Self {
            status: VideoStatus::InProgress,
            updated_at: now,
            ..self
        }
    }

    pub fn mark_downloaded(self, now: DateTime<Utc>) -> Self {
        Self {
            status: VideoStatus::Downloaded,
            updated_at: now,
            ..self
        }
    }

    pub fn mark_errored_retrying(self, now: DateTime<Utc>) -> Self {
        Self {
            status: VideoStatus::ErroredRetrying,
            updated_at: now,
            ..self
        }
    }

    pub fn mark_errored(self, now: DateTime<Utc>) -> Self {
        Self {
            status: VideoStatus::Errored,
            updated_at: now,
            ..self
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

    fn video() -> Video {
        Video::create(
            PlaylistId::new("PL1").unwrap(),
            VideoId::new("vid1").unwrap(),
            "My Video",
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
    }

    #[test]
    fn it_should_transition_to_in_progress_when_a_download_starts() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().start_download(now);

        assert_eq!(video.status, VideoStatus::InProgress);
        assert_eq!(video.updated_at, now);
    }

    #[test]
    fn it_should_transition_to_downloaded_when_marked_downloaded() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_downloaded(now);

        assert_eq!(video.status, VideoStatus::Downloaded);
        assert_eq!(video.updated_at, now);
    }

    #[test]
    fn it_should_transition_to_errored_retrying_when_marked_errored_retrying() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_errored_retrying(now);

        assert_eq!(video.status, VideoStatus::ErroredRetrying);
        assert_eq!(video.updated_at, now);
    }

    #[test]
    fn it_should_transition_to_errored_when_marked_errored() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_errored(now);

        assert_eq!(video.status, VideoStatus::Errored);
        assert_eq!(video.updated_at, now);
    }
}
