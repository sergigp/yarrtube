use crate::domain::shared::Quality;
use crate::domain::video::Video;
use crate::domain::video::VideoRecordId;
use crate::domain::video::thumbnail_filename::expected_thumbnail_filename;
use crate::domain::video::top_level_entry;
use crate::domain::video::video_filename::VideoFilename;
use crate::domain::video_metadata::{VideoMetadata, build_video_metadata, resolve_sorttitle};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_metadata_repository::YoutubeMetadataRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::{
    DownloadAttempt, DownloadedVideo, VideoDownloaderRepository,
};
use crate::infrastructure::shared::system_clock::Clock;
use std::path::Path;
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
    youtube_metadata_repository: Arc<dyn YoutubeMetadataRepository>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    clock: Arc<dyn Clock>,
}

impl VideoDownloader {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        youtube_metadata_repository: Arc<dyn YoutubeMetadataRepository>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_repository,
            video_downloader_repository,
            video_file_repository,
            playlist_video_repository,
            youtube_metadata_repository,
            video_metadata_repository,
            clock,
        }
    }
}

pub trait VideoDownloaderApi: Send + Sync {
    /// No-ops (without touching status) if the video no longer exists,
    /// since that means the download no longer needs to happen. A video
    /// deleted while its download ran gets nothing recorded, and the folder
    /// the download wrote is removed. Returns
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
        match self.run_download(&started, quality, output_dir, metadata.as_ref()) {
            Ok(DownloadAttempt::Succeeded(downloaded)) => {
                log_sabr(&started, downloaded.sabr_notice.as_deref());
                self.record_unless_deleted(started, downloaded, quality, output_dir, metadata)
            }
            Ok(DownloadAttempt::Failed {
                stderr,
                sabr_notice,
            }) => {
                log_sabr(&started, sabr_notice.as_deref());
                self.record_failed(started, stderr, is_last_attempt)
            }
            Err(e) => self.record_errored(started, e, is_last_attempt),
        }
    }
}

impl VideoDownloader {
    /// Resolves the video's folder before `yt-dlp` runs, reusing the one a
    /// thumbnail fetched ahead of the download already created, if any, and
    /// writes `movie.nfo` into it first, so a media scanner never sees the
    /// video without it. When the download doesn't succeed, that
    /// `movie.nfo` is removed, and so is a folder freshly created for this
    /// attempt, so a retry's collision check reuses the same folder name
    /// instead of suffixing it.
    fn run_download(
        &self,
        video: &Video,
        quality: Quality,
        output_dir: &Path,
        metadata: Option<&VideoMetadata>,
    ) -> anyhow::Result<DownloadAttempt> {
        let filename = VideoFilename::from_title(&video.title);
        let existing_folder = video.thumbnail_filename.as_deref().map(top_level_entry);
        let folder = self.video_downloader_repository.prepare_folder(
            filename.as_str(),
            video.youtube_id.as_str(),
            output_dir,
            existing_folder,
        )?;
        let video_dir = output_dir.join(&folder);
        if let Some(metadata) = metadata {
            self.write_nfo_ahead(video, metadata, &video_dir, &folder);
        }
        info!(video_id = %video.id, "downloading video");
        let attempt = self.video_downloader_repository.download(
            &video.youtube_id.to_url(),
            filename.as_str(),
            video.youtube_id.as_str(),
            quality,
            output_dir,
            Some(&folder),
        );
        if !matches!(attempt, Ok(DownloadAttempt::Succeeded(_))) {
            self.remove_nfo(video, &video_dir);
            if existing_folder.is_none() {
                self.remove_fresh_folder(video, output_dir, &folder);
            }
        }
        attempt
    }

    /// Best-effort: without it the video still downloads, and Plex then
    /// imports it before its `movie.nfo` exists, as before this step.
    fn write_nfo_ahead(
        &self,
        video: &Video,
        metadata: &VideoMetadata,
        video_dir: &Path,
        folder: &str,
    ) {
        let ahead = metadata.clone().with_thumb(Some(format!("{folder}.jpg")));
        if let Err(e) = self.video_metadata_repository.write_nfo(&ahead, video_dir) {
            warn!(video_id = %video.id, error = %e, "failed to write movie.nfo ahead of the download");
        }
    }

    /// Best-effort: a stale `movie.nfo` next to no media is ignored by Plex
    /// and overwritten by the next attempt.
    fn remove_nfo(&self, video: &Video, video_dir: &Path) {
        if let Err(e) = self.video_metadata_repository.remove_nfo(video_dir) {
            warn!(video_id = %video.id, error = %e, "failed to remove movie.nfo of a download that did not succeed");
        }
    }

    /// Best-effort: a leftover folder only costs the retry a suffixed name.
    fn remove_fresh_folder(&self, video: &Video, output_dir: &Path, folder: &str) {
        if let Err(e) = self.video_file_repository.delete(output_dir, folder) {
            warn!(video_id = %video.id, error = %e, "failed to remove the folder of a download that did not succeed");
        }
    }

    /// The video (or its whole playlist/channel) may have been deleted while
    /// it downloaded: then the folder the download wrote is removed instead
    /// of recorded, so no stray folder outlives it.
    fn record_unless_deleted(
        &self,
        started: Video,
        downloaded: DownloadedVideo,
        quality: Quality,
        output_dir: &Path,
        metadata: Option<VideoMetadata>,
    ) -> anyhow::Result<()> {
        if self.video_repository.find(&started.id)?.is_none() {
            self.discard_download(&started, &downloaded, output_dir);
            return Ok(());
        }
        self.record_downloaded(started, downloaded, quality, output_dir, metadata)
    }

    /// Best-effort: a folder that can't be removed is logged, and the next
    /// reconcile's orphan sweep (if the container still exists) removes it.
    fn discard_download(&self, video: &Video, downloaded: &DownloadedVideo, output_dir: &Path) {
        info!(video_id = %video.id, folder = %downloaded.folder, "video deleted during its download, removing the downloaded folder");
        if let Err(e) = self
            .video_file_repository
            .delete(output_dir, &downloaded.folder)
        {
            warn!(video_id = %video.id, error = %e, "failed to remove the folder of a download whose video was deleted");
        }
    }

    fn record_downloaded(
        &self,
        started: Video,
        downloaded: DownloadedVideo,
        quality: Quality,
        output_dir: &Path,
        metadata: Option<VideoMetadata>,
    ) -> anyhow::Result<()> {
        let video_dir = output_dir.join(&downloaded.folder);
        let thumbnail_filename = self.find_downloaded_thumbnail(&video_dir, &downloaded)?;
        let filename = format!("{}/{}", downloaded.folder, downloaded.filename);
        let downloaded_video = started.mark_downloaded(
            quality,
            filename,
            thumbnail_filename.clone(),
            downloaded.duration_seconds,
            self.clock.now(),
        );
        self.video_repository.update(&downloaded_video)?;
        info!(video_id = %downloaded_video.id, "video downloaded");
        let thumb_basename = thumbnail_filename
            .as_deref()
            .and_then(|f| Path::new(f).file_name())
            .and_then(|f| f.to_str());
        self.save_metadata(&downloaded_video, &video_dir, thumb_basename, metadata);
        Ok(())
    }

    /// The `<folder>/<thumbnail>` path of the thumbnail `yt-dlp` wrote next
    /// to the downloaded video, if it wrote one.
    fn find_downloaded_thumbnail(
        &self,
        video_dir: &Path,
        downloaded: &DownloadedVideo,
    ) -> anyhow::Result<Option<String>> {
        let expected_thumbnail = expected_thumbnail_filename(&downloaded.filename);
        Ok(self
            .video_file_repository
            .list(video_dir)?
            .iter()
            .any(|f| f == &expected_thumbnail)
            .then(|| format!("{}/{}", downloaded.folder, expected_thumbnail)))
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
        let excluded = started.mark_excluded(self.clock.now());
        self.video_repository.update(&excluded)?;
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

    fn mark_errored(&self, started: Video, is_last_attempt: bool) -> anyhow::Result<()> {
        let updated = if is_last_attempt {
            started.mark_errored(self.clock.now())
        } else {
            started.mark_errored_retrying(self.clock.now())
        };
        self.video_repository.update(&updated)
    }

    /// Fetches `video`'s YouTube metadata and resolves its `sorttitle` (no
    /// `thumb` yet) — see design.md's "Failure handling: skip the save
    /// entirely, never fail the download" decision. Any failure (the YouTube
    /// fetch or the playlist-position lookup) is logged and swallowed rather
    /// than propagated: metadata generation never fails or retries the
    /// download itself, and a skipped/failed attempt self-heals on the next
    /// reconcile pass (see `PlaylistVideoReconciler`/`ChannelVideoReconciler`).
    fn fetch_metadata(&self, video: &Video) -> Option<VideoMetadata> {
        let metadata = match self.youtube_metadata_repository.find(&video.youtube_id) {
            Ok(Some(metadata)) => metadata,
            Ok(None) => {
                warn!(video_id = %video.id, "no YouTube metadata found for video, skipping metadata generation");
                return None;
            }
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to fetch YouTube metadata, skipping metadata generation");
                return None;
            }
        };

        let playlist_position = match self.playlist_video_repository.find_by_video(&video.id) {
            Ok(playlist_video) => playlist_video.and_then(|pv| pv.position),
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to look up playlist position, falling back to publish-date sorttitle");
                None
            }
        };
        let sorttitle =
            resolve_sorttitle(&metadata.title, metadata.published_at, playlist_position);
        Some(build_video_metadata(
            &video.youtube_id,
            &metadata,
            sorttitle,
            None,
            self.clock.now(),
        ))
    }

    /// Saves the downloaded `video`'s `movie.nfo` and records its metadata,
    /// referencing the thumbnail actually downloaded. When the fetch before
    /// the download failed, it is attempted once more here. A failure is
    /// logged and swallowed, never failing the download.
    fn save_metadata(
        &self,
        video: &Video,
        video_dir: &Path,
        thumbnail_filename: Option<&str>,
        metadata: Option<VideoMetadata>,
    ) {
        let Some(metadata) = metadata.or_else(|| self.fetch_metadata(video)) else {
            return;
        };
        let video_metadata = metadata.with_thumb(thumbnail_filename.map(str::to_string));
        if let Err(e) = self
            .video_metadata_repository
            .save(&video.id, &video_metadata, video_dir)
        {
            warn!(video_id = %video.id, error = %e, "failed to save video metadata");
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
