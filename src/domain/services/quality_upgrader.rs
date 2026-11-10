use crate::domain::event::DomainEvent;
use crate::domain::services::{
    VideoOwnerFinder, VideoOwnerFinderApi, VideoUpdater, VideoUpdaterApi,
};
use crate::domain::shared::Quality;
use crate::domain::video::{
    DownloadAttempt, DownloadedVideo, Video, VideoDownloaded, VideoId, VideoStepOutcome,
    VideoTarget,
};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::VideoDownloaderRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::staging_area::StagingArea;
use crate::infrastructure::shared::system_clock::Clock;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Why a check the SABR-only streaming experiment degraded runs again.
const DEGRADED_REASON: &str = "YouTube's SABR-only streaming degraded the quality check";

/// Upgrades a young downloaded video to the better quality YouTube finished
/// processing after its download — see the `video-download` capability's
/// "Young Video Quality Upgrade". Runs from the `upgrade_quality` task;
/// `VideoReconciler` decides when to schedule it.
pub struct QualityUpgrader {
    video_repository: Arc<dyn VideoRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
    staging_area: Arc<StagingArea>,
    video_updater: VideoUpdater,
}

impl QualityUpgrader {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
        staging_area: Arc<StagingArea>,
    ) -> Self {
        Self {
            video_updater: VideoUpdater::new(video_repository.clone()),
            video_repository,
            video_owner_finder,
            video_downloader_repository,
            video_file_repository,
            event_publisher,
            clock,
            staging_area,
        }
    }
}

pub trait QualityUpgraderApi: Send + Sync {
    /// No-op unless the video is quality-checkable
    /// (`Video::is_quality_checkable`). Asks the downloader which height
    /// its owner's quality would download now; when above the recorded one,
    /// re-downloads into the video's staging dir and replaces its episode
    /// file in the owner's show directory in place, then records the new tier and height
    /// and publishes `VideoDownloaded`. Every check is recorded under the
    /// `late-video-data` policy: a failed, degraded or no-better probe or
    /// re-download is a failed attempt, re-queued under the `video-retries`
    /// policy. Fails only when the video can't be read or stored.
    fn upgrade(&self, video_id: &VideoId) -> anyhow::Result<VideoStepOutcome>;
}

impl QualityUpgraderApi for QualityUpgrader {
    fn upgrade(&self, video_id: &VideoId) -> anyhow::Result<VideoStepOutcome> {
        let Some((video, target)) = self.find_checkable(video_id)? else {
            return Ok(VideoStepOutcome::Settled);
        };
        let staged = self.staging_area.for_video(&video.id);
        match self.check_and_upgrade(&video, target.quality, staged.path(), &target.output_dir) {
            Ok(()) => self.retry_if_degraded(&video),
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to check the video's quality");
                self.record_failed_check(&video, e.to_string())
            }
        }
    }
}

impl QualityUpgrader {
    /// The video to check with its owner's target; `None` when the video
    /// is gone or not quality-checkable, or (logged) when no playlist or
    /// channel holds it any more.
    fn find_checkable(&self, video_id: &VideoId) -> anyhow::Result<Option<(Video, VideoTarget)>> {
        let Some(video) = self.video_repository.find(video_id)? else {
            debug!(video_id = %video_id, "video no longer exists, skipping quality check");
            return Ok(None);
        };
        if !video.is_quality_checkable() {
            return Ok(None);
        }
        let Some(owner) = self.video_owner_finder.find(&video)? else {
            info!(video_id = %video_id, "video has no playlist or channel any more, skipping quality check");
            return Ok(None);
        };
        Ok(Some((video, owner.target)))
    }

    /// Probes the height `quality` would download now: re-downloads when
    /// it is above the recorded one, else records the check — a failed one
    /// when the probe was degraded (SABR), since its height proves nothing.
    fn check_and_upgrade(
        &self,
        video: &Video,
        quality: Quality,
        staging: &Path,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let probe = self
            .video_downloader_repository
            .probe_quality(&video.youtube_id.to_url(), quality)?;
        let published_at = probe.published_at.unwrap_or(video.created_at);
        if probe.height > video.downloaded_height {
            self.redownload(video, quality, staging, output_dir, published_at)
        } else {
            let probed_height = probe.height.filter(|_| !probe.degraded);
            self.record_check_on(video, probed_height, published_at)
                .map(|_| ())
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
        staging: &Path,
        output_dir: &Path,
        published_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let attempt = self.video_downloader_repository.download(
            &video.youtube_id.to_url(),
            quality,
            staging,
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
            DownloadAttempt::Failed { failure, .. } => Err(anyhow!(
                "yt-dlp failed to download a higher quality: {}",
                Some(failure.reason())
                    .filter(|reason| !reason.is_empty())
                    .unwrap_or("no reason given")
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
        let now = self.clock.now();
        let upgraded = self.video_updater.update(&video.id, |video| {
            Some(video.record_downloaded_quality(
                quality,
                downloaded.height,
                downloaded.sabr_notice.is_some(),
                published_at,
                now,
            ))
        })?;
        if let Some(upgraded) = upgraded {
            self.publish_downloaded(&upgraded, output_dir);
        }
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

    /// The video as stored once the check is recorded on it.
    fn record_check_on(
        &self,
        video: &Video,
        probed_height: Option<u32>,
        published_at: DateTime<Utc>,
    ) -> anyhow::Result<Option<Video>> {
        let now = self.clock.now();
        self.video_updater.update(&video.id, |video| {
            Some(video.record_quality_check(probed_height, published_at, now))
        })
    }

    /// A check that completed but was degraded by SABR left the quality
    /// Retrying: it runs again when its next attempt is due.
    fn retry_if_degraded(&self, video: &Video) -> anyhow::Result<VideoStepOutcome> {
        let stored = self.video_repository.find(&video.id)?;
        Ok(VideoStepOutcome::retry_at_or_settled(
            stored.and_then(|video| video.quality_checks.state.next_attempt_at()),
            DEGRADED_REASON.to_string(),
        ))
    }

    /// Records a failed check, failed for `reason`: Retrying, run again when
    /// its next attempt is due, or Failed.
    fn record_failed_check(
        &self,
        video: &Video,
        reason: String,
    ) -> anyhow::Result<VideoStepOutcome> {
        let recorded = self.record_check_on(video, None, video.created_at)?;
        Ok(VideoStepOutcome::retry_at_or_settled(
            recorded.and_then(|video| video.quality_checks.state.next_attempt_at()),
            reason,
        ))
    }

    /// Best effort, like `VideoDownloader`'s: the upgrade is already
    /// recorded, only reacting to it (e.g. a Plex scan) is lost.
    fn publish_downloaded(&self, video: &Video, output_dir: &Path) {
        let Some(number) = video.episode_number else {
            return;
        };
        let event =
            DomainEvent::VideoDownloaded(VideoDownloaded::new(&video.id, output_dir, &number));
        if let Err(e) = self.event_publisher.publish(&event) {
            warn!(video_id = %video.id, error = %e, "failed to publish that the video was upgraded");
        }
    }
}
