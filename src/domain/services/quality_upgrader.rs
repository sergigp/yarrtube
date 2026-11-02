use crate::domain::event::DomainEvent;
use crate::domain::shared::Quality;
use crate::domain::video::{Video, VideoDownloaded, VideoRecordId, staging_dir};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::{
    DownloadAttempt, DownloadedVideo, VideoDownloaderRepository,
};
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::path::Path;
use std::sync::Arc;
use tracing::warn;

/// Upgrades a young downloaded video to the better quality YouTube finished
/// processing after its download — see the `video-download` capability's
/// "Young Video Quality Upgrade". Runs from the `upgrade_quality` task;
/// `VideoReconciler` decides when to schedule it.
#[derive(Clone)]
pub struct QualityUpgrader {
    video_repository: Arc<dyn VideoRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl QualityUpgrader {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_repository,
            video_downloader_repository,
            video_file_repository,
            event_publisher,
            clock,
            videos_path: videos_path.into(),
        }
    }
}

pub trait QualityUpgraderApi: Send + Sync {
    /// No-op unless the video is quality-checkable
    /// (`Video::is_quality_checkable`). Asks the downloader which height
    /// `quality` would download now; when above the recorded one,
    /// re-downloads into the video's staging dir and replaces its episode
    /// file in `output_dir` in place, then records the new tier and height
    /// and publishes `VideoDownloaded`. Every check is recorded under the
    /// `late-video-data` policy. Best effort: a failed probe or re-download
    /// is recorded as a failed check, never returned.
    fn upgrade(
        &self,
        video_id: &VideoRecordId,
        quality: Quality,
        output_dir: &Path,
    ) -> anyhow::Result<()>;
}

impl QualityUpgraderApi for QualityUpgrader {
    fn upgrade(
        &self,
        video_id: &VideoRecordId,
        quality: Quality,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let Some(video) = self.video_repository.find(video_id)? else {
            return Ok(());
        };
        let probe = self
            .video_downloader_repository
            .probe_quality(&video.youtube_id.to_url(), quality)?;
        let published_at = probe.published_at.unwrap_or(video.created_at);
        if probe.height > video.downloaded_height {
            self.redownload(&video, quality, output_dir, published_at)?;
        }
        self.discard_staging(&video);
        Ok(())
    }
}

impl QualityUpgrader {
    /// Downloads `video` again at `quality` into its staging dir and
    /// replaces its episode file with it.
    fn redownload(
        &self,
        video: &Video,
        quality: Quality,
        output_dir: &Path,
        published_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let attempt = self.video_downloader_repository.download(
            &video.youtube_id.to_url(),
            quality,
            &staging_dir(&self.videos_path, &video.id),
        )?;
        match attempt {
            DownloadAttempt::Succeeded(downloaded) => {
                self.replace(video, &downloaded, quality, output_dir, published_at)
            }
            DownloadAttempt::Failed { .. } => Ok(()),
        }
    }

    /// Moves the re-downloaded media over the episode file, then records
    /// only the quality fields and announces the new file.
    fn replace(
        &self,
        video: &Video,
        downloaded: &DownloadedVideo,
        quality: Quality,
        output_dir: &Path,
        published_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let Some(filename) = video.filename.as_deref() else {
            return Ok(());
        };
        self.video_file_repository
            .move_file(&downloaded.media_path, &output_dir.join(filename))?;
        let upgraded = video.clone().record_downloaded_quality(
            quality,
            downloaded.height,
            published_at,
            self.clock.now(),
        );
        self.video_repository.update_quality(&upgraded)?;
        self.publish_downloaded(&upgraded, output_dir);
        Ok(())
    }

    /// Best effort, like `VideoDownloader`'s: the upgrade is already
    /// recorded, only reacting to it (e.g. a Plex scan) is lost.
    fn publish_downloaded(&self, video: &Video, output_dir: &Path) {
        let Some(number) = video.episode_number else {
            return;
        };
        let event = DomainEvent::VideoDownloaded(VideoDownloaded {
            video_id: video.id.as_str().to_string(),
            output_dir: output_dir.to_string_lossy().to_string(),
            folder: number.season_folder(),
        });
        if let Err(e) = self.event_publisher.publish(&event) {
            warn!(video_id = %video.id, error = %e, "failed to publish that the video was upgraded");
        }
    }

    /// Best effort: a leftover staging dir is harmless.
    fn discard_staging(&self, video: &Video) {
        let dir = staging_dir(&self.videos_path, &video.id);
        if let Err(e) = self.video_file_repository.delete_dir_recursive(&dir) {
            warn!(video_id = %video.id, error = %e, "failed to discard the staging dir");
        }
    }
}
