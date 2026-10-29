use crate::domain::video::EpisodePaths;
use crate::domain::video::Video;
use crate::domain::video::staging_dir;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::{
    FetchedThumbnail, ThumbnailFetch, VideoDownloaderRepository,
};
use crate::infrastructure::shared::system_clock::Clock;
use std::path::Path;
use std::sync::Arc;
use tracing::warn;

/// Best-effort thumbnail fetch, independent of and ahead of a video's full
/// download — see the `video-thumbnails` capability. The fetch itself runs
/// from the `fetch_thumbnail` task and never returns an error to its caller;
/// `VideoReconciler` decides when to schedule it.
#[derive(Clone)]
pub struct ThumbnailFetcher {
    video_repository: Arc<dyn VideoRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl ThumbnailFetcher {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_repository,
            video_downloader_repository,
            video_file_repository,
            clock,
            videos_path: videos_path.into(),
        }
    }
}

pub trait ThumbnailFetcherApi: Send + Sync {
    /// No-ops if `video` already has a recorded thumbnail or is no longer
    /// thumbnail-fetchable (`Video::is_thumbnail_fetchable`). Otherwise fetches
    /// one into the video's staging dir, assigns its episode number and moves
    /// it into `output_dir`'s season folder as the episode image, recording
    /// only the thumbnail; any failure (a clean "no thumbnail available"
    /// outcome, or a systemic error) is logged and swallowed, the staging dir
    /// discarded, leaving `video` untouched.
    fn fetch(&self, video: &Video, output_dir: &Path);
}

impl ThumbnailFetcherApi for ThumbnailFetcher {
    fn fetch(&self, video: &Video, output_dir: &Path) {
        if video.thumbnail_filename.is_some() || !video.is_thumbnail_fetchable() {
            return;
        }

        self.fetch_and_place(video, output_dir);
        self.discard_staging(video);
    }
}

impl ThumbnailFetcher {
    /// Fetches and places the thumbnail; every failure is logged.
    fn fetch_and_place(&self, video: &Video, output_dir: &Path) {
        match self.fetch_thumbnail(video, output_dir) {
            Ok(ThumbnailFetch::Fetched(fetched)) => {
                if let Err(e) = self.place_thumbnail(video, fetched, output_dir) {
                    warn!(video_id = %video.id, error = %e, "failed to place fetched thumbnail");
                }
            }
            Ok(ThumbnailFetch::Unavailable { reason }) => {
                warn!(video_id = %video.id, reason = reason.as_deref(), "no thumbnail available for video");
            }
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to fetch video thumbnail");
            }
        }
    }

    fn fetch_thumbnail(&self, video: &Video, _output_dir: &Path) -> anyhow::Result<ThumbnailFetch> {
        self.video_downloader_repository.fetch_thumbnail(
            &video.youtube_id.to_url(),
            &staging_dir(&self.videos_path, &video.id),
        )
    }

    /// Assigns the episode number, moves the staged jpg to the episode's
    /// image path and writes only the thumbnail: a download of the same
    /// video may have changed its status since `video` was read.
    fn place_thumbnail(
        &self,
        video: &Video,
        fetched: FetchedThumbnail,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let published_at = fetched.published_at.unwrap_or_else(|| self.clock.now());
        let number = self
            .video_repository
            .assign_episode_number(&video.id, published_at)?;
        let thumbnail_filename = EpisodePaths::new(&number, &video.title).thumbnail();
        self.video_file_repository
            .move_file(&fetched.path, &output_dir.join(&thumbnail_filename))?;
        self.video_repository
            .update_thumbnail(&video.id, &thumbnail_filename, self.clock.now())
    }

    /// Best-effort: a leftover staging dir is harmless, Plex and Kodi ignore
    /// the staging root.
    fn discard_staging(&self, video: &Video) {
        let dir = staging_dir(&self.videos_path, &video.id);
        if let Err(e) = self.video_file_repository.delete_dir_recursive(&dir) {
            warn!(video_id = %video.id, error = %e, "failed to discard the staging dir");
        }
    }
}
