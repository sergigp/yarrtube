use crate::domain::services::{VideoOwnerFinder, VideoOwnerFinderApi};
use crate::domain::video::EpisodePaths;
use crate::domain::video::{FetchedThumbnail, ThumbnailFetch};
use crate::domain::video::{Video, VideoId};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_thumbnail_repository::ThumbnailRepository;
use crate::infrastructure::shared::staging_area::StagingArea;
use crate::infrastructure::shared::system_clock::Clock;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Best-effort thumbnail fetch, independent of and ahead of a video's full
/// download — see the `video-thumbnails` capability. The fetch itself runs
/// from the `fetch_thumbnail` task and never returns an error to its caller;
/// `VideoReconciler` decides when to schedule it.
#[derive(Clone)]
pub struct ThumbnailFetcher {
    video_repository: Arc<dyn VideoRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    thumbnail_repository: Arc<dyn ThumbnailRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    clock: Arc<dyn Clock>,
    staging_area: Arc<StagingArea>,
}

impl ThumbnailFetcher {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        thumbnail_repository: Arc<dyn ThumbnailRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        clock: Arc<dyn Clock>,
        staging_area: Arc<StagingArea>,
    ) -> Self {
        Self {
            video_repository,
            video_owner_finder,
            thumbnail_repository,
            video_file_repository,
            clock,
            staging_area,
        }
    }
}

pub trait ThumbnailFetcherApi: Send + Sync {
    /// No-ops if the video, or the playlist or channel holding it (logged),
    /// no longer exists, or the video already has a recorded thumbnail or is
    /// no longer thumbnail-fetchable (`Video::is_thumbnail_fetchable`).
    /// Otherwise fetches one into the video's staging dir, assigns its
    /// episode number and moves it into its owner's show directory's season
    /// folder as the episode image, recording only the thumbnail; any fetch
    /// failure (a clean "no thumbnail available" outcome, or a systemic
    /// error) is logged and swallowed, the staging dir discarded, leaving the
    /// video untouched. Fails only when the video or its owner can't be read.
    fn fetch(&self, video_id: &VideoId) -> anyhow::Result<()>;
}

impl ThumbnailFetcherApi for ThumbnailFetcher {
    fn fetch(&self, video_id: &VideoId) -> anyhow::Result<()> {
        let Some(video) = self.video_repository.find(video_id)? else {
            debug!(video_id = %video_id, "video no longer exists, skipping thumbnail fetch");
            return Ok(());
        };
        if video.thumbnail_filename.is_some() || !video.is_thumbnail_fetchable() {
            return Ok(());
        }
        let Some(owner) = self.video_owner_finder.find(&video)? else {
            info!(video_id = %video_id, "video has no playlist or channel any more, skipping thumbnail fetch");
            return Ok(());
        };
        let staged = self.staging_area.for_video(&video.id);
        self.fetch_and_place(&video, staged.path(), &owner.target.output_dir);
        Ok(())
    }
}

impl ThumbnailFetcher {
    /// Fetches and places the thumbnail; every failure is logged.
    fn fetch_and_place(&self, video: &Video, staging: &Path, output_dir: &Path) {
        match self.fetch_thumbnail(video, staging) {
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

    fn fetch_thumbnail(&self, video: &Video, staging: &Path) -> anyhow::Result<ThumbnailFetch> {
        self.thumbnail_repository
            .fetch_thumbnail(&video.youtube_id.to_url(), staging)
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
}
