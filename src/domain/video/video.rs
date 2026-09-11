use super::video_status::VideoStatus;
use crate::domain::shared::{PlaylistId, Quality, VideoId};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    pub playlist_id: PlaylistId,
    pub video_id: VideoId,
    pub title: String,
    pub status: VideoStatus,
    pub quality: Option<Quality>,
    pub filename: Option<String>,
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
            quality: None,
            filename: None,
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

    pub fn mark_downloaded(
        self,
        quality: Quality,
        filename: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            status: VideoStatus::Downloaded,
            quality: Some(quality),
            filename: Some(filename.into()),
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

    /// Resets a `Downloaded` video back to `Pending`, clearing its recorded
    /// filename and quality, used by filesystem reconciliation when a
    /// video's recorded file is missing from disk and needs redownloading.
    pub fn reset_for_redownload(self, now: DateTime<Utc>) -> Self {
        Self {
            status: VideoStatus::Pending,
            quality: None,
            filename: None,
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
        assert_eq!(video.quality, None);
        assert_eq!(video.filename, None);
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
        assert_eq!(video.quality, None);
        assert_eq!(video.updated_at, now);
    }

    #[test]
    fn it_should_transition_to_downloaded_when_marked_downloaded() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_downloaded(Quality::High, "My Video.mp4", now);

        assert_eq!(video.status, VideoStatus::Downloaded);
        assert_eq!(video.quality, Some(Quality::High));
        assert_eq!(video.filename, Some("My Video.mp4".to_string()));
        assert_eq!(video.updated_at, now);
    }

    #[test]
    fn it_should_reset_a_downloaded_video_back_to_pending_for_redownload() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let downloaded = video().mark_downloaded(Quality::High, "My Video.mp4", now);

        let later = DateTime::<Utc>::from_timestamp(200, 0).unwrap();
        let reset = downloaded.reset_for_redownload(later);

        assert_eq!(reset.status, VideoStatus::Pending);
        assert_eq!(reset.quality, None);
        assert_eq!(reset.filename, None);
        assert_eq!(reset.updated_at, later);
    }

    #[test]
    fn it_should_transition_to_errored_retrying_when_marked_errored_retrying() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_errored_retrying(now);

        assert_eq!(video.status, VideoStatus::ErroredRetrying);
        assert_eq!(video.quality, None);
        assert_eq!(video.updated_at, now);
    }

    #[test]
    fn it_should_transition_to_errored_when_marked_errored() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_errored(now);

        assert_eq!(video.status, VideoStatus::Errored);
        assert_eq!(video.quality, None);
        assert_eq!(video.updated_at, now);
    }
}
