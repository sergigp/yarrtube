use crate::domain::shared::Quality;
use crate::domain::video::Video;
use crate::domain::video::VideoRecordId;
use crate::domain::video::thumbnail_filename::expected_thumbnail_filename;
use crate::domain::video::top_level_entry;
use crate::domain::video::video_filename::VideoFilename;
use crate::domain::video_metadata::{build_video_metadata, resolve_sorttitle};
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
    /// since that means the download no longer needs to happen. Returns
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

        let started = video.start_download(self.clock.now());
        self.video_repository.update(&started)?;
        match self.run_download(&started, quality, output_dir) {
            Ok(DownloadAttempt::Succeeded(downloaded)) => {
                self.record_downloaded(started, downloaded, quality, output_dir)
            }
            Ok(DownloadAttempt::Failed { stderr }) => {
                self.record_failed(started, stderr, is_last_attempt)
            }
            Err(e) => self.record_errored(started, e, is_last_attempt),
        }
    }
}

impl VideoDownloader {
    /// Reuses the folder a thumbnail fetched ahead of the download already
    /// created, if any.
    fn run_download(
        &self,
        video: &Video,
        quality: Quality,
        output_dir: &Path,
    ) -> anyhow::Result<DownloadAttempt> {
        let filename = VideoFilename::from_title(&video.title);
        let existing_folder = video.thumbnail_filename.as_deref().map(top_level_entry);
        info!(video_id = %video.id, "downloading video");
        self.video_downloader_repository.download(
            &video.youtube_id.to_url(),
            filename.as_str(),
            video.youtube_id.as_str(),
            quality,
            output_dir,
            existing_folder,
        )
    }

    fn record_downloaded(
        &self,
        started: Video,
        downloaded: DownloadedVideo,
        quality: Quality,
        output_dir: &Path,
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
        self.generate_metadata(&downloaded_video, &video_dir, thumb_basename);
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

    /// A clean `yt-dlp` failure: marks the video errored and returns
    /// `yt-dlp`'s reported error so the task queue retries/dead-letters it.
    fn record_failed(
        &self,
        started: Video,
        stderr: Option<String>,
        is_last_attempt: bool,
    ) -> anyhow::Result<()> {
        let video_id = started.id.clone();
        self.mark_errored(started, is_last_attempt)?;
        let error_message =
            stderr.unwrap_or_else(|| format!("yt-dlp failed to download video {video_id}"));
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

    /// Fetches `video`'s YouTube metadata, resolves its `sorttitle`, and
    /// saves its `movie.nfo` — see design.md's "Failure handling: skip the
    /// save entirely, never fail the download" decision. Any failure
    /// anywhere in this sequence (the YouTube fetch, the playlist-position
    /// lookup, or the save itself) is logged and swallowed rather than
    /// propagated: metadata generation never fails or retries the download
    /// itself, and a skipped/failed attempt self-heals on the next
    /// reconcile pass (see `PlaylistVideoReconciler`/`ChannelVideoReconciler`).
    fn generate_metadata(&self, video: &Video, video_dir: &Path, thumbnail_filename: Option<&str>) {
        let metadata = match self.youtube_metadata_repository.find(&video.youtube_id) {
            Ok(Some(metadata)) => metadata,
            Ok(None) => {
                warn!(video_id = %video.id, "no YouTube metadata found for video, skipping metadata generation");
                return;
            }
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to fetch YouTube metadata, skipping metadata generation");
                return;
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
        let video_metadata = build_video_metadata(
            &video.youtube_id,
            &metadata,
            sorttitle,
            thumbnail_filename.map(str::to_string),
        );

        if let Err(e) = self
            .video_metadata_repository
            .save(&video.id, &video_metadata, video_dir)
        {
            warn!(video_id = %video.id, error = %e, "failed to save video metadata");
        }
    }
}
