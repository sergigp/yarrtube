use crate::domain::shared::{Quality, VideoRecordId};
use crate::domain::video::video_filename::VideoFilename;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::VideoDownloaderRepository;
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
    clock: Arc<dyn Clock>,
}

impl VideoDownloader {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_repository,
            video_downloader_repository,
            clock,
        }
    }

    /// No-ops (without touching status) if the video no longer exists,
    /// since that means the download no longer needs to happen. Returns
    /// `Err` on a failed download so the task queue retries/dead-letters it.
    pub fn download(
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

        let filename = VideoFilename::from_title(&video.title);
        let started = video.start_download(self.clock.now());
        self.video_repository.update(&started)?;

        info!(video_id = %video_id, "downloading video");
        let outcome = self.video_downloader_repository.download(
            &started.youtube_id.to_url(),
            filename.as_str(),
            started.youtube_id.as_str(),
            quality,
            output_dir,
        );

        match outcome {
            Ok(Some(downloaded_filename)) => {
                self.video_repository.update(&started.mark_downloaded(
                    quality,
                    downloaded_filename,
                    self.clock.now(),
                ))?;
                info!(video_id = %video_id, "video downloaded");
                Ok(())
            }
            Ok(None) => {
                let updated = if is_last_attempt {
                    started.mark_errored(self.clock.now())
                } else {
                    started.mark_errored_retrying(self.clock.now())
                };
                self.video_repository.update(&updated)?;
                warn!(video_id = %video_id, "yt-dlp reported a failed download");
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
                error!(video_id = %video_id, error = %e, "video download errored");
                Err(e)
            }
        }
    }
}
