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
use anyhow::anyhow;
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
        if !video.is_quality_checkable() {
            return Ok(());
        }
        if let Err(e) = self.check_and_upgrade(&video, quality, output_dir) {
            warn!(video_id = %video.id, error = %e, "failed to check the video's quality");
            self.record_failed_check(&video);
        }
        self.discard_staging(&video);
        Ok(())
    }
}

impl QualityUpgrader {
    /// Probes the height `quality` would download now: re-downloads when
    /// it is above the recorded one, else records the check — a failed one
    /// when the probe was degraded (SABR), since its height proves nothing.
    fn check_and_upgrade(
        &self,
        video: &Video,
        quality: Quality,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let probe = self
            .video_downloader_repository
            .probe_quality(&video.youtube_id.to_url(), quality)?;
        let published_at = probe.published_at.unwrap_or(video.created_at);
        if probe.height > video.downloaded_height {
            self.redownload(video, quality, output_dir, published_at)
        } else {
            let probed_height = probe.height.filter(|_| !probe.degraded);
            self.record_check(video, probed_height, published_at)
        }
    }

    /// Downloads `video` again at `quality` into its staging dir and
    /// replaces its episode file with it, only when the download is taller
    /// than the recorded one: probe and download are separate yt-dlp runs,
    /// and YouTube may serve the second a worse format list (e.g. SABR).
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
            DownloadAttempt::Succeeded(downloaded)
                if downloaded.height > video.downloaded_height =>
            {
                self.replace(video, &downloaded, quality, output_dir, published_at)
            }
            DownloadAttempt::Succeeded(downloaded) => Err(anyhow!(
                "yt-dlp downloaded no higher quality ({:?}) than the recorded one ({:?})",
                downloaded.height,
                video.downloaded_height
            )),
            DownloadAttempt::Failed { stderr, .. } => Err(anyhow!(
                "yt-dlp failed to download a higher quality: {}",
                stderr.as_deref().unwrap_or("no reason given")
            )),
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
        if !self.is_still_downloaded_as(video)? {
            return Ok(());
        }
        self.video_file_repository
            .move_file(&downloaded.media_path, &output_dir.join(filename))?;
        let upgraded = video.clone().record_downloaded_quality(
            quality,
            downloaded.height,
            downloaded.sabr_notice.is_some(),
            published_at,
            self.clock.now(),
        );
        self.video_repository.update_quality(&upgraded)?;
        self.publish_downloaded(&upgraded, output_dir);
        Ok(())
    }

    /// Whether the stored video is still Downloaded as the same file the
    /// upgrade started with: it may have been deleted, or reset for
    /// redownload, while the upgrade downloaded.
    fn is_still_downloaded_as(&self, video: &Video) -> anyhow::Result<bool> {
        Ok(self
            .video_repository
            .find(&video.id)?
            .is_some_and(|stored| stored.is_downloaded() && stored.filename == video.filename))
    }

    /// Records a quality check that found nothing better (`probed_height`)
    /// or failed (`None`).
    fn record_check(
        &self,
        video: &Video,
        probed_height: Option<u32>,
        published_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        self.video_repository
            .update_quality(&video.clone().record_quality_check(
                probed_height,
                published_at,
                self.clock.now(),
            ))
    }

    /// Best effort, like the check itself: a failure to record the failed
    /// check is only logged.
    fn record_failed_check(&self, video: &Video) {
        if let Err(e) = self.record_check(video, None, video.created_at) {
            warn!(video_id = %video.id, error = %e, "failed to record a failed quality check");
        }
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
