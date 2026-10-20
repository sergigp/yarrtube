use crate::domain::event::DomainEvent;
use crate::domain::services::{MetadataGenerator, MetadataGeneratorApi};
use crate::domain::shared::Quality;
use crate::domain::video::{
    EpisodeNumber, EpisodePaths, Video, VideoDownloaded, VideoRecordId, staging_dir,
};
use crate::domain::video_metadata::VideoMetadata;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::{
    DownloadAttempt, DownloadedVideo, VideoDownloaderRepository,
};
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Substrings that, found (case-insensitively) in a diagnosed failure reason,
/// mark a video permanently unavailable — see design.md's token table. This
/// is an allowlist of *permanent* reasons, never an inverted "everything
/// except transient" rule, so an unseen `yt-dlp`/YouTube wording is never
/// silently excluded forever.
const PERMANENT_UNAVAILABILITY_TOKENS: [&str; 8] = [
    "members-only",
    "claimed content",
    "copyright",
    "in your country",
    "private video",
    "has been removed",
    "no longer available",
    "account associated with this video has been terminated",
];

/// Whether a diagnosed failure `reason` names a block that can never succeed
/// on a later attempt, so the video should be excluded rather than retried.
fn is_permanently_unavailable_reason(reason: &str) -> bool {
    let reason = reason.to_lowercase();
    PERMANENT_UNAVAILABILITY_TOKENS
        .iter()
        .any(|token| reason.contains(token))
}

/// Joins the diagnosed reason (the richer signal) with the download's own
/// stderr (the fallback) for logging and classification, skipping whichever
/// is absent or empty.
fn combined_reason(diagnosed: Option<&str>, stderr: Option<&str>) -> String {
    [diagnosed, stderr]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" | ")
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

/// Downloads one video via `yt-dlp`, transitioning it through in-progress to
/// downloaded/errored. Container-agnostic: the caller (a task handler)
/// already resolved the video's owning container's quality and output
/// directory before invoking this.
#[derive(Clone)]
pub struct VideoDownloader {
    video_repository: Arc<dyn VideoRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    metadata_generator: Arc<MetadataGenerator>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl VideoDownloader {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        metadata_generator: Arc<MetadataGenerator>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_repository,
            video_downloader_repository,
            video_file_repository,
            playlist_video_repository,
            metadata_generator,
            video_metadata_repository,
            event_publisher,
            clock,
            videos_path: videos_path.into(),
        }
    }
}

pub trait VideoDownloaderApi: Send + Sync {
    /// No-ops (without touching status) if the video no longer exists,
    /// since that means the download no longer needs to happen. Downloads
    /// into the video's staging dir, then places the files as an episode in
    /// `output_dir`'s season folder. A video deleted while its download ran
    /// gets nothing recorded, and its staging dir is discarded. Returns
    /// `Err` on a failed download so the task queue retries/dead-letters it.
    fn download(
        &self,
        video_id: VideoRecordId,
        quality: Quality,
        output_dir: &Path,
        is_last_attempt: bool,
    ) -> anyhow::Result<()>;
}

impl VideoDownloaderApi for VideoDownloader {
    fn download(
        &self,
        video_id: VideoRecordId,
        quality: Quality,
        output_dir: &Path,
        is_last_attempt: bool,
    ) -> anyhow::Result<()> {
        let Some(video) = self.video_repository.find(&video_id)? else {
            debug!(video_id = %video_id, "video no longer exists, skipping download");
            return Ok(());
        };
        if video.is_download_settled() {
            debug!(video_id = %video_id, status = video.status.as_str(), "video already settled, skipping download");
            return Ok(());
        }

        let started = video.start_download(self.clock.now());
        self.video_repository.update(&started)?;
        let metadata = self.fetch_metadata(&started);
        match self.run_download(&started, quality, output_dir) {
            Ok(DownloadAttempt::Succeeded(downloaded)) => {
                log_sabr(&started, downloaded.sabr_notice.as_deref());
                self.record_unless_deleted(
                    started.clone(),
                    downloaded,
                    quality,
                    output_dir,
                    metadata,
                )
                .or_else(|e| self.record_placement_errored(started, e, is_last_attempt))
            }
            Ok(DownloadAttempt::Failed {
                stderr,
                sabr_notice,
            }) => {
                log_sabr(&started, sabr_notice.as_deref());
                self.discard_staging(&started);
                self.record_failed(started, stderr, is_last_attempt)
            }
            Err(e) => {
                self.discard_staging(&started);
                self.record_errored(started, e, is_last_attempt)
            }
        }
    }
}

impl VideoDownloader {
    fn run_download(
        &self,
        video: &Video,
        quality: Quality,
        _output_dir: &Path,
    ) -> anyhow::Result<DownloadAttempt> {
        info!(video_id = %video.id, "downloading video");
        self.video_downloader_repository.download(
            &video.youtube_id.to_url(),
            quality,
            &staging_dir(&self.videos_path, &video.id),
        )
    }

    /// The video (or its whole playlist/channel) may have been deleted while
    /// it downloaded: then its staging dir is discarded instead of placed,
    /// so nothing outlives it.
    fn record_unless_deleted(
        &self,
        started: Video,
        downloaded: DownloadedVideo,
        quality: Quality,
        output_dir: &Path,
        metadata: Option<VideoMetadata>,
    ) -> anyhow::Result<()> {
        if self.video_repository.find(&started.id)?.is_none() {
            self.discard_staging(&started);
            return Ok(());
        }
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
        let downloaded_video =
            self.record_downloaded(video, &downloaded, &files, quality, number)?;
        self.save_metadata(&downloaded_video, metadata, &number, &files);
        self.discard_staging(&downloaded_video);
        self.publish_downloaded(&downloaded_video, output_dir, &number.season_folder());
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

    fn record_downloaded(
        &self,
        video: Video,
        downloaded: &DownloadedVideo,
        files: &EpisodeFiles,
        quality: Quality,
        number: EpisodeNumber,
    ) -> anyhow::Result<Video> {
        let now = self.clock.now();
        let downloaded_video = video
            .mark_downloaded(
                quality,
                files.filename.clone(),
                files.thumbnail_filename.clone(),
                downloaded.duration_seconds,
                now,
            )
            .with_episode_number(number, now);
        let downloaded_video = self.save(downloaded_video)?;
        info!(video_id = %downloaded_video.id, "video downloaded");
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
    fn publish_downloaded(&self, video: &Video, output_dir: &Path, folder: &str) {
        let event = DomainEvent::VideoDownloaded(VideoDownloaded {
            video_id: video.id.as_str().to_string(),
            output_dir: output_dir.to_string_lossy().to_string(),
            folder: folder.to_string(),
        });
        if let Err(e) = self.event_publisher.publish(&event) {
            warn!(video_id = %video.id, error = %e, "failed to publish that the video was downloaded");
        }
    }

    /// Best-effort: removes the video's staging dir and whatever a download
    /// or thumbnail fetch left in it; a leftover is harmless, Plex and Kodi
    /// ignore the staging root.
    fn discard_staging(&self, video: &Video) {
        let dir = staging_dir(&self.videos_path, &video.id);
        if let Err(e) = self.video_file_repository.delete_dir_recursive(&dir) {
            warn!(video_id = %video.id, error = %e, "failed to discard the staging dir");
        }
    }

    /// A clean `yt-dlp` failure: diagnoses the precise reason (see design.md),
    /// logs it, and either excludes a permanently-unavailable video (settling
    /// the task with `Ok(())`, so it is neither retried nor dead-lettered) or
    /// keeps the current errored/retry behavior and returns `Err`.
    fn record_failed(
        &self,
        started: Video,
        stderr: Option<String>,
        is_last_attempt: bool,
    ) -> anyhow::Result<()> {
        let diagnosed = self.diagnose_failure(&started.youtube_id.to_url());
        let reason = combined_reason(diagnosed.as_deref(), stderr.as_deref());
        if is_permanently_unavailable_reason(&reason) {
            self.exclude(started, &reason)
        } else {
            self.fail_retryably(started, reason, is_last_attempt)
        }
    }

    /// Diagnoses a failed download's precise reason, swallowing a probe error
    /// (logged) as undetermined so the probe failing never fails the handling.
    fn diagnose_failure(&self, video_url: &str) -> Option<String> {
        match self.video_downloader_repository.diagnose(video_url) {
            Ok(reason) => reason,
            Err(e) => {
                warn!(error = %e, "diagnostic probe failed; treating the failure reason as undetermined");
                None
            }
        }
    }

    /// Marks a permanently-unavailable video excluded and settles the task
    /// without error, so no retry is scheduled and it is not dead-lettered.
    fn exclude(&self, started: Video, reason: &str) -> anyhow::Result<()> {
        let video_id = started.id.clone();
        self.save(started.mark_excluded(self.clock.now()))?;
        info!(video_id = %video_id, reason = %reason, "excluding permanently-unavailable video");
        Ok(())
    }

    /// Marks the video errored (retrying, or permanently on the last attempt)
    /// and returns the precise reason so the task queue retries/dead-letters
    /// it.
    fn fail_retryably(
        &self,
        started: Video,
        reason: String,
        is_last_attempt: bool,
    ) -> anyhow::Result<()> {
        let video_id = started.id.clone();
        self.mark_errored(started, is_last_attempt)?;
        let error_message = if reason.is_empty() {
            format!("yt-dlp failed to download video {video_id}")
        } else {
            reason
        };
        warn!(video_id = %video_id, error = %error_message, "yt-dlp reported a failed download");
        Err(anyhow::anyhow!(error_message))
    }

    /// A systemic download error: marks the video errored and propagates
    /// the error so the task queue retries/dead-letters it.
    fn record_errored(
        &self,
        started: Video,
        error: anyhow::Error,
        is_last_attempt: bool,
    ) -> anyhow::Result<()> {
        let video_id = started.id.clone();
        self.mark_errored(started, is_last_attempt)?;
        error!(video_id = %video_id, error = %error, "video download errored");
        Err(error)
    }

    /// Placing a successful download failed (numbering, moving or recording
    /// it): discards the staging dir and errors the video like a failed
    /// download, so it settles as errored instead of staying in progress.
    /// Re-reads the video so a number assigned before the failure is kept; a
    /// video deleted meanwhile gets nothing recorded.
    fn record_placement_errored(
        &self,
        started: Video,
        error: anyhow::Error,
        is_last_attempt: bool,
    ) -> anyhow::Result<()> {
        self.discard_staging(&started);
        match self.video_repository.find(&started.id)? {
            Some(current) => self.record_errored(current, error, is_last_attempt),
            None => Err(error),
        }
    }

    fn mark_errored(&self, started: Video, is_last_attempt: bool) -> anyhow::Result<()> {
        let updated = if is_last_attempt {
            started.mark_errored(self.clock.now())
        } else {
            started.mark_errored_retrying(self.clock.now())
        };
        self.save(updated).map(|_| ())
    }

    /// Saves `video` with the watch state stored now, not the one read when
    /// the download started; a video gone meanwhile is saved as given.
    fn save(&self, video: Video) -> anyhow::Result<Video> {
        let saved = match self.video_repository.find(&video.id)? {
            Some(current) => video.with_watch_state_of(&current),
            None => video,
        };
        self.video_repository.update(&saved)?;
        Ok(saved)
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
}

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

#[cfg(test)]
mod tests {
    use super::is_permanently_unavailable_reason;

    #[test]
    fn it_should_classify_permanently_unavailable_reasons() {
        let reasons = [
            "Join this channel to get access to members-only content like this video, and other exclusive perks.",
            "It was blocked due to the claimed content by Mediatoon.",
            "Video unavailable. This video contains content that has been blocked on copyright grounds.",
            "The uploader has not made this video available in your country.",
            "Private video. Sign in if you've been granted access to this video.",
            "This video has been removed by the uploader.",
            "This video is no longer available.",
            "This video is not available because the YouTube account associated with this video has been terminated.",
        ];

        assert_eq!(
            reasons.map(is_permanently_unavailable_reason),
            [true, true, true, true, true, true, true, true]
        );
    }

    #[test]
    fn it_should_classify_permanently_unavailable_reasons_case_insensitively() {
        let reasons = [
            "JOIN THIS CHANNEL FOR MEMBERS-ONLY CONTENT",
            "It was blocked due to the CLAIMED CONTENT by X.",
        ];

        assert_eq!(reasons.map(is_permanently_unavailable_reason), [true, true]);
    }

    #[test]
    fn it_should_not_classify_generic_or_transient_reasons() {
        let reasons = [
            "Video unavailable",
            "HTTP Error 403: Forbidden",
            "Unable to download webpage: The read operation timed out",
            "HTTP Error 429: Too Many Requests",
            "Sign in to confirm you're not a bot",
            "Unable to download video data: fragment 3 not found",
            "Connection reset by peer",
        ];

        assert_eq!(reasons.map(is_permanently_unavailable_reason), [false; 7]);
    }
}
