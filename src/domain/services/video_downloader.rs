use crate::domain::shared::{PlaylistId, Quality, VideoId};
use crate::domain::video::video_filename::VideoFilename;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::VideoDownloaderRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Downloads one video via `yt-dlp`, transitioning it through in-progress to
/// downloaded/errored.
#[derive(Clone)]
pub struct VideoDownloader {
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl VideoDownloader {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_repository: Arc<dyn VideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            playlist_repository,
            video_repository,
            video_downloader_repository,
            clock,
            videos_path: videos_path.into(),
        }
    }

    /// No-ops (without touching status) if the playlist or the video no
    /// longer exist, since either means the download no longer needs to
    /// happen. Returns `Err` on a failed download so the task queue
    /// retries/dead-letters it.
    pub fn download(
        &self,
        playlist_id: PlaylistId,
        video_id: VideoId,
        quality: Quality,
        is_last_attempt: bool,
    ) -> anyhow::Result<()> {
        let Some(playlist) = self.playlist_repository.find(&playlist_id)? else {
            debug!(playlist_id = %playlist_id, "playlist no longer exists, skipping download");
            return Ok(());
        };
        let Some(video) = self.video_repository.find(&playlist_id, &video_id)? else {
            debug!(playlist_id = %playlist_id, video_id = %video_id, "video no longer exists, skipping download");
            return Ok(());
        };

        let filename = VideoFilename::from_title(&video.title);
        let started = video.start_download(self.clock.now());
        self.video_repository.update(&started)?;

        let output_dir = Path::new(&self.videos_path).join(playlist.path.as_str());
        info!(playlist_id = %playlist_id, video_id = %video_id, "downloading video");
        let outcome = self.video_downloader_repository.download(
            &video_id.to_url(),
            filename.as_str(),
            video_id.as_str(),
            quality,
            &output_dir,
        );

        match outcome {
            Ok(Some(downloaded_filename)) => {
                self.video_repository.update(&started.mark_downloaded(
                    quality,
                    downloaded_filename,
                    self.clock.now(),
                ))?;
                info!(playlist_id = %playlist_id, video_id = %video_id, "video downloaded");
                Ok(())
            }
            Ok(None) => {
                let updated = if is_last_attempt {
                    started.mark_errored(self.clock.now())
                } else {
                    started.mark_errored_retrying(self.clock.now())
                };
                self.video_repository.update(&updated)?;
                warn!(playlist_id = %playlist_id, video_id = %video_id, "yt-dlp reported a failed download");
                Err(anyhow::anyhow!(
                    "yt-dlp failed to download video {video_id}"
                ))
            }
            Err(e) => {
                let updated = if is_last_attempt {
                    started.mark_errored(self.clock.now())
                } else {
                    started.mark_errored_retrying(self.clock.now())
                };
                self.video_repository.update(&updated)?;
                error!(playlist_id = %playlist_id, video_id = %video_id, error = %e, "video download errored");
                Err(e)
            }
        }
    }
}
