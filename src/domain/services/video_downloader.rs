use crate::domain::event::DomainEvent;
use crate::domain::services::{
    MetadataGenerator, MetadataGeneratorApi, VideoOwnerFinder, VideoOwnerFinderApi, VideoUpdater,
    VideoUpdaterApi,
};
use crate::domain::shared::Quality;
use crate::domain::video::{
    DownloadAttempt, DownloadFailure, DownloadedVideo, EpisodeNumber, EpisodePaths, Video,
    VideoDownloaded, VideoId, VideoStatus, VideoStepOutcome, VideoTarget,
};
use crate::domain::video_metadata::VideoMetadata;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::VideoDownloaderRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::staging_area::StagingArea;
use crate::infrastructure::shared::system_clock::Clock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Where a downloaded video's files go in its show: the recorded media and
/// thumbnail paths (relative to the show dir), the episode NFO's path, and the
/// thumbnail's name as the NFO references it.
struct EpisodeFiles {
    filename: String,
    thumbnail_filename: Option<String>,
    nfo_path: PathBuf,
    thumb: Option<String>,
}

impl EpisodeFiles {
    fn new(
        number: &EpisodeNumber,
        title: &str,
        downloaded: &DownloadedVideo,
        output_dir: &Path,
    ) -> Self {
        let extension = downloaded
            .media_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("mp4");
        let has_thumbnail = downloaded.thumbnail_path.is_some();
        let paths = EpisodePaths::new(number, title);
        Self {
            filename: paths.media(extension),
            thumbnail_filename: has_thumbnail.then(|| paths.thumbnail()),
            nfo_path: output_dir.join(paths.nfo()),
            thumb: has_thumbnail.then(|| paths.thumb()),
        }
    }
}

/// Downloads one video via `yt-dlp`, transitioning it through in-progress to
/// downloaded/errored, at its owner's quality and into its owner's show
/// directory as they are when the download runs.
pub struct VideoDownloader {
    video_repository: Arc<dyn VideoRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    metadata_generator: Arc<MetadataGenerator>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
    staging_area: Arc<StagingArea>,
    video_updater: VideoUpdater,
}

impl VideoDownloader {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        metadata_generator: Arc<MetadataGenerator>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
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
            playlist_video_repository,
            metadata_generator,
            video_metadata_repository,
            event_publisher,
            clock,
            staging_area,
        }
    }
}

pub trait VideoDownloaderApi: Send + Sync {
    /// No-ops (without touching status) if the video, or the playlist or
    /// channel holding it, no longer exists, since that means the download no
    /// longer needs to happen. Downloads at the owner's quality into the
    /// video's staging dir, then places the files as an episode in the
    /// owner's show directory's season folder. A video deleted while its download ran
    /// gets nothing recorded, and its staging dir is discarded. A failed
    /// download is recorded on the video under the `video-retries` policy.
    fn download(&self, video_id: VideoId) -> anyhow::Result<VideoStepOutcome>;
}

impl VideoDownloaderApi for VideoDownloader {
    fn download(&self, video_id: VideoId) -> anyhow::Result<VideoStepOutcome> {
        let Some((video, target)) = self.find_downloadable(&video_id)? else {
            return Ok(VideoStepOutcome::Settled);
        };
        if let Some(waiting) = self.wait_for_next_attempt(&video) {
            return Ok(waiting);
        }
        let Some(started) = self.start_download(&video.id)? else {
            return Ok(VideoStepOutcome::Settled);
        };
        let metadata = self.fetch_metadata(&started);
        let staged = self.staging_area.for_video(&started.id);
        let attempt = self.run_download(&started, target.quality, staged.path());
        self.record_attempt(started, attempt, &target, metadata)
    }
}

impl VideoDownloader {
    /// The video to download with its owner's target; `None` (logged) when
    /// the video is gone, already settled, or no playlist or channel holds
    /// it any more.
    fn find_downloadable(
        &self,
        video_id: &VideoId,
    ) -> anyhow::Result<Option<(Video, VideoTarget)>> {
        let Some(video) = self.video_repository.find(video_id)? else {
            debug!(video_id = %video_id, "video no longer exists, skipping download");
            return Ok(None);
        };
        if Self::is_settled(&video) {
            debug!(video_id = %video_id, status = video.status.as_str(), "video already settled, skipping download");
            return Ok(None);
        }
        let Some(owner) = self.video_owner_finder.find(&video)? else {
            info!(video_id = %video_id, "video has no playlist or channel any more, skipping download");
            return Ok(None);
        };
        Ok(Some((video, owner.target)))
    }

    /// A Retrying download whose next attempt isn't due yet runs again when
    /// it is, downloading nothing now.
    fn wait_for_next_attempt(&self, video: &Video) -> Option<VideoStepOutcome> {
        let at = video
            .next_download_at()
            .filter(|at| *at > self.clock.now())?;
        debug!(video_id = %video.id, next_attempt_at = %at, "download not due yet, re-queueing it");
        Some(VideoStepOutcome::RetryAt {
            at,
            reason: video.download_failure.clone().unwrap_or_default(),
        })
    }

    /// The video as stored once its download started; `None` (logged) when
    /// it is gone or its download is already settled.
    fn start_download(&self, video_id: &VideoId) -> anyhow::Result<Option<Video>> {
        let now = self.clock.now();
        let started = self.video_updater.update(video_id, |video| {
            (!Self::is_settled(&video)).then(|| video.start_download(now))
        })?;
        if started.is_none() {
            debug!(video_id = %video_id, "video gone or already settled, skipping download");
        }
        Ok(started)
    }

    fn run_download(
        &self,
        video: &Video,
        quality: Quality,
        staging: &Path,
    ) -> anyhow::Result<DownloadAttempt> {
        info!(video_id = %video.id, "downloading video");
        self.video_downloader_repository
            .download(&video.youtube_id.to_url(), quality, staging)
    }

    /// Records the download's outcome: places a successful one as an
    /// episode, makes a failed one Unavailable or records a failed attempt.
    fn record_attempt(
        &self,
        started: Video,
        attempt: anyhow::Result<DownloadAttempt>,
        target: &VideoTarget,
        metadata: Option<VideoMetadata>,
    ) -> anyhow::Result<VideoStepOutcome> {
        match attempt {
            Ok(DownloadAttempt::Succeeded(downloaded)) => {
                Self::log_sabr(&started, downloaded.sabr_notice.as_deref());
                self.record_unless_deleted(started.clone(), downloaded, target, metadata)
                    .map(|()| VideoStepOutcome::Settled)
                    .or_else(|e| self.record_errored(started, e))
            }
            Ok(DownloadAttempt::Failed {
                failure,
                sabr_notice,
            }) => {
                Self::log_sabr(&started, sabr_notice.as_deref());
                self.record_failed(started, failure)
            }
            Err(e) => self.record_errored(started, e),
        }
    }

    /// The video (or its whole playlist/channel) may have been deleted while
    /// it downloaded: then nothing is placed, and `download` discards its
    /// staging dir like on every other outcome, so nothing outlives it. This
    /// re-read can't reuse `find_downloadable`'s: the download in between
    /// runs for minutes. The outcome is then recorded through `VideoUpdater`,
    /// so watch state recorded while the files were placed isn't lost either.
    fn record_unless_deleted(
        &self,
        started: Video,
        downloaded: DownloadedVideo,
        target: &VideoTarget,
        metadata: Option<VideoMetadata>,
    ) -> anyhow::Result<()> {
        if self.video_repository.find(&started.id)?.is_none() {
            return Ok(());
        }
        let (quality, output_dir) = (target.quality, target.output_dir.as_path());
        self.place_download(started, downloaded, quality, output_dir, metadata)
    }

    /// Assigns the episode number, writes the episode NFO at its final path,
    /// moves media and thumbnail into the season folder, records the video
    /// and publishes `VideoDownloaded`.
    fn place_download(
        &self,
        video: Video,
        downloaded: DownloadedVideo,
        quality: Quality,
        output_dir: &Path,
        metadata: Option<VideoMetadata>,
    ) -> anyhow::Result<()> {
        let number = self.assign_episode_number(&video, &downloaded)?;
        let files = EpisodeFiles::new(&number, &video.title, &downloaded, output_dir);
        let metadata = metadata.map(|metadata| metadata.with_thumb(files.thumb.clone()));
        if let Some(metadata) = &metadata {
            self.write_nfo(&video, metadata, &number, &files.nfo_path);
        }
        self.move_into_place(&downloaded, output_dir, &files)?;
        let Some(downloaded_video) =
            self.record_downloaded(&video.id, &downloaded, &files, quality, number)?
        else {
            return Ok(());
        };
        self.save_metadata(&downloaded_video, metadata, &number, &files);
        self.publish_downloaded(&downloaded_video, output_dir, &number);
        Ok(())
    }

    /// The number recorded for the video, or a fresh one from the publish
    /// time `yt-dlp` reported (now, when it reported none).
    fn assign_episode_number(
        &self,
        video: &Video,
        downloaded: &DownloadedVideo,
    ) -> anyhow::Result<EpisodeNumber> {
        let published_at = downloaded.published_at.unwrap_or_else(|| self.clock.now());
        self.video_repository
            .assign_episode_number(&video.id, published_at)
    }

    /// The video as stored once downloaded; `None` if it was deleted while
    /// its files were placed.
    fn record_downloaded(
        &self,
        video_id: &VideoId,
        downloaded: &DownloadedVideo,
        files: &EpisodeFiles,
        quality: Quality,
        number: EpisodeNumber,
    ) -> anyhow::Result<Option<Video>> {
        let now = self.clock.now();
        let downloaded_video = self.video_updater.update(video_id, |video| {
            let published_at = downloaded.published_at.unwrap_or(video.created_at);
            let downloaded_video = video
                .mark_downloaded(
                    quality,
                    files.filename.clone(),
                    files.thumbnail_filename.clone(),
                    downloaded.duration_seconds,
                    now,
                )
                .with_episode_number(number, now)
                .record_downloaded_quality(
                    quality,
                    downloaded.height,
                    downloaded.sabr_notice.is_some(),
                    published_at,
                    now,
                );
            Some(match &downloaded.chapters {
                Some(chapters) => downloaded_video.record_chapters_check(
                    Some(chapters.clone()),
                    published_at,
                    now,
                ),
                None => downloaded_video,
            })
        })?;
        if downloaded_video.is_some() {
            info!(video_id = %video_id, "video downloaded");
        }
        Ok(downloaded_video)
    }

    /// Best-effort: without it the episode still lands, and Plex imports it
    /// without its NFO until `save_metadata` or reconcile writes one.
    fn write_nfo(
        &self,
        video: &Video,
        metadata: &VideoMetadata,
        number: &EpisodeNumber,
        nfo_path: &Path,
    ) {
        if let Err(e) = self
            .video_metadata_repository
            .write_nfo(metadata, number, nfo_path)
        {
            warn!(video_id = %video.id, error = %e, "failed to write the episode NFO");
        }
    }

    /// Records the downloaded video's metadata (rewriting its NFO), fetching
    /// it once more when the fetch before the download failed. A failure is
    /// logged and swallowed, never failing the download; reconcile
    /// regenerates it.
    fn save_metadata(
        &self,
        video: &Video,
        metadata: Option<VideoMetadata>,
        number: &EpisodeNumber,
        files: &EpisodeFiles,
    ) {
        let Some(metadata) = metadata.or_else(|| {
            self.fetch_metadata(video)
                .map(|metadata| metadata.with_thumb(files.thumb.clone()))
        }) else {
            return;
        };
        if let Err(e) =
            self.video_metadata_repository
                .save(&video.id, &metadata, number, &files.nfo_path)
        {
            warn!(video_id = %video.id, error = %e, "failed to save video metadata");
        }
    }

    fn move_into_place(
        &self,
        downloaded: &DownloadedVideo,
        output_dir: &Path,
        files: &EpisodeFiles,
    ) -> anyhow::Result<()> {
        self.video_file_repository
            .move_file(&downloaded.media_path, &output_dir.join(&files.filename))?;
        match (&downloaded.thumbnail_path, &files.thumbnail_filename) {
            (Some(from), Some(to)) => self
                .video_file_repository
                .move_file(from, &output_dir.join(to)),
            _ => Ok(()),
        }
    }

    /// Best-effort: the download is already recorded, and losing the event
    /// only skips reacting to it (e.g. asking Plex to scan the show).
    fn publish_downloaded(&self, video: &Video, output_dir: &Path, number: &EpisodeNumber) {
        let event =
            DomainEvent::VideoDownloaded(VideoDownloaded::new(&video.id, output_dir, number));
        if let Err(e) = self.event_publisher.publish(&event) {
            warn!(video_id = %video.id, error = %e, "failed to publish that the video was downloaded");
        }
    }

    /// A clean `yt-dlp` failure: either makes a permanently-unavailable
    /// video Unavailable (settling the task) or records a failed attempt.
    fn record_failed(
        &self,
        started: Video,
        failure: DownloadFailure,
    ) -> anyhow::Result<VideoStepOutcome> {
        if failure.is_permanent() {
            self.mark_unavailable(started, &failure)
                .map(|()| VideoStepOutcome::Settled)
        } else {
            self.fail_retryably(started, &failure)
        }
    }

    /// Marks a permanently-unavailable video Unavailable and settles the
    /// task without error, so no retry is scheduled.
    fn mark_unavailable(&self, started: Video, failure: &DownloadFailure) -> anyhow::Result<()> {
        let video_id = started.id.clone();
        let now = self.clock.now();
        self.video_updater
            .update(&video_id, |video| Some(video.mark_unavailable(now)))?;
        info!(video_id = %video_id, reason = %failure.reason(), "marking permanently-unavailable video unavailable");
        Ok(())
    }

    /// Records a failed attempt with `yt-dlp`'s reason.
    fn fail_retryably(
        &self,
        started: Video,
        failure: &DownloadFailure,
    ) -> anyhow::Result<VideoStepOutcome> {
        let video_id = started.id.clone();
        let error_message = failure.message_for(&video_id);
        let errored = self.mark_errored(started, &error_message)?;
        warn!(video_id = %video_id, error = %error_message, "yt-dlp reported a failed download");
        Ok(Self::next_attempt(errored, error_message))
    }

    /// When a Retrying download runs again; settled once it gave up (or is
    /// gone).
    fn next_attempt(errored: Option<Video>, reason: String) -> VideoStepOutcome {
        match errored.and_then(|video| video.next_download_at()) {
            Some(at) => VideoStepOutcome::RetryAt { at, reason },
            None => VideoStepOutcome::Settled,
        }
    }

    /// The download couldn't run at all (e.g. `yt-dlp` failed to start) or
    /// its files couldn't be placed (numbering, moving or recording them): a
    /// failed attempt like any other, with that error as its reason. Applied
    /// to the video as stored, so a number assigned before the failure is
    /// kept; a video deleted meanwhile gets nothing recorded.
    fn record_errored(
        &self,
        started: Video,
        error: anyhow::Error,
    ) -> anyhow::Result<VideoStepOutcome> {
        error!(video_id = %started.id, error = %error, "video download errored");
        let reason = error.to_string();
        let errored = self.mark_errored(started, &reason)?;
        Ok(Self::next_attempt(errored, reason))
    }

    fn mark_errored(&self, started: Video, reason: &str) -> anyhow::Result<Option<Video>> {
        let now = self.clock.now();
        self.video_updater
            .update(&started.id, |video| Some(video.mark_errored(reason, now)))
    }

    /// Whether the video's download has nothing left to do: Downloaded,
    /// Failed or Unavailable.
    fn is_settled(video: &Video) -> bool {
        matches!(
            video.status,
            VideoStatus::Downloaded | VideoStatus::Failed | VideoStatus::Unavailable
        )
    }

    /// Builds `video`'s metadata with its playlist position (no `thumb` yet)
    /// — see design.md's "Failure handling: skip the save entirely, never
    /// fail the download" decision. Any failure (the playlist-position lookup
    /// or the YouTube fetch) is logged and swallowed rather than propagated:
    /// metadata generation never fails or retries the download itself, and a
    /// skipped/failed attempt self-heals on the next reconcile pass (see
    /// `InternalVideoReconciler`).
    fn fetch_metadata(&self, video: &Video) -> Option<VideoMetadata> {
        let playlist_position = match self.playlist_video_repository.find_by_video(&video.id) {
            Ok(playlist_video) => playlist_video.map(|pv| pv.position),
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to look up playlist position, falling back to publish-date sorttitle");
                None
            }
        };
        self.metadata_generator
            .generate(video, playlist_position, None)
    }

    /// Emits the dedicated, greppable SABR event when `yt-dlp` reported YouTube's
    /// SABR-only streaming experiment for this download (see the `video-download`
    /// capability's "SABR-Only Streaming Is Surfaced" requirement). It is purely
    /// observational: it fires on both a failed and a degraded-successful download
    /// and never changes the video's status, recorded reason, or retry flow.
    fn log_sabr(video: &Video, sabr_notice: Option<&str>) {
        if let Some(reason) = sabr_notice {
            warn!(
                video_id = %video.id,
                reason = %reason,
                "SABR-only streaming experiment reported by yt-dlp; higher-quality formats were skipped"
            );
        }
    }
}
