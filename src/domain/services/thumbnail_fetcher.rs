use crate::domain::services::{
    VideoOwnerFinder, VideoOwnerFinderApi, VideoUpdater, VideoUpdaterApi,
};
use crate::domain::video::EpisodePaths;
use crate::domain::video::{FetchedThumbnail, ThumbnailFetch, VideoStepOutcome};
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
pub struct ThumbnailFetcher {
    video_repository: Arc<dyn VideoRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    thumbnail_repository: Arc<dyn ThumbnailRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    clock: Arc<dyn Clock>,
    staging_area: Arc<StagingArea>,
    video_updater: VideoUpdater,
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
            video_updater: VideoUpdater::new(video_repository.clone()),
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
    /// folder as the episode image, recording only the thumbnail; a fetch
    /// that obtains none (a clean "no thumbnail available" outcome, or a
    /// systemic error) is logged and counted as a failed attempt under the
    /// `video-retries` policy, the staging dir discarded. Fails only when
    /// the video or its owner can't be read or stored.
    fn fetch(&self, video_id: &VideoId) -> anyhow::Result<VideoStepOutcome>;
}

impl ThumbnailFetcherApi for ThumbnailFetcher {
    fn fetch(&self, video_id: &VideoId) -> anyhow::Result<VideoStepOutcome> {
        let Some(video) = self.video_repository.find(video_id)? else {
            debug!(video_id = %video_id, "video no longer exists, skipping thumbnail fetch");
            return Ok(VideoStepOutcome::Settled);
        };
        if video.thumbnail_filename.is_some() || !video.is_thumbnail_fetchable() {
            return Ok(VideoStepOutcome::Settled);
        }
        let Some(owner) = self.video_owner_finder.find(&video)? else {
            info!(video_id = %video_id, "video has no playlist or channel any more, skipping thumbnail fetch");
            return Ok(VideoStepOutcome::Settled);
        };
        let staged = self.staging_area.for_thumbnail(&video.id);
        match self.fetch_and_place(&video, staged.path(), &owner.target.output_dir) {
            Ok(()) => Ok(VideoStepOutcome::Settled),
            Err(reason) => self.record_failure(&video, reason),
        }
    }
}

impl ThumbnailFetcher {
    /// Fetches and places the thumbnail; every failure is logged and
    /// returned as its reason.
    fn fetch_and_place(
        &self,
        video: &Video,
        staging: &Path,
        output_dir: &Path,
    ) -> Result<(), String> {
        match self.fetch_thumbnail(video, staging) {
            Ok(ThumbnailFetch::Fetched(fetched)) => self
                .place_thumbnail(video, fetched, output_dir)
                .map_err(|e| {
                    warn!(video_id = %video.id, error = %e, "failed to place fetched thumbnail");
                    e.to_string()
                }),
            Ok(ThumbnailFetch::Unavailable { reason }) => {
                warn!(video_id = %video.id, reason = reason.as_deref(), "no thumbnail available for video");
                Err(reason.unwrap_or_else(|| "no thumbnail available".to_string()))
            }
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to fetch video thumbnail");
                Err(e.to_string())
            }
        }
    }

    /// Records a fetch that obtained no thumbnail on the video as stored
    /// (unless it got one or stopped wanting one meanwhile): Retrying, run
    /// again when its next attempt is due, or Failed.
    fn record_failure(&self, video: &Video, reason: String) -> anyhow::Result<VideoStepOutcome> {
        let now = self.clock.now();
        let recorded = self.video_updater.update(&video.id, |video| {
            (video.thumbnail_filename.is_none() && video.is_thumbnail_fetchable())
                .then(|| video.record_thumbnail_failure(now))
        })?;
        Ok(VideoStepOutcome::retry_at_or_settled(
            recorded.and_then(|video| video.thumbnail.next_attempt_at()),
            reason,
        ))
    }

    fn fetch_thumbnail(&self, video: &Video, staging: &Path) -> anyhow::Result<ThumbnailFetch> {
        self.thumbnail_repository
            .fetch_thumbnail(&video.youtube_id.to_url(), staging)
    }

    /// Assigns the episode number, moves the staged jpg to the episode's
    /// image path and records the thumbnail on the video as stored now: a
    /// download of the same video may have changed it since `video` was read.
    ///
    /// The fetch can take a while, and the video (or its whole
    /// playlist/channel) may have been deleted, excluded, errored or already
    /// given a thumbnail meanwhile: then nothing is placed, so a stale
    /// thumbnail isn't dropped into a show folder being torn down. Mirrors
    /// `SubtitleFetcher`/`QualityUpgrader`'s re-read before placing.
    fn place_thumbnail(
        &self,
        video: &Video,
        fetched: FetchedThumbnail,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let Some(stored) = self.still_placeable(video)? else {
            return Ok(());
        };
        let published_at = fetched.published_at.unwrap_or_else(|| self.clock.now());
        let number = self
            .video_repository
            .assign_episode_number(&stored.id, published_at)?;
        let thumbnail_filename = EpisodePaths::new(&number, &stored.title).thumbnail();
        self.video_file_repository
            .move_file(&fetched.path, &output_dir.join(&thumbnail_filename))?;
        let now = self.clock.now();
        self.video_updater
            .update(&stored.id, |video| {
                Some(video.with_thumbnail(thumbnail_filename.clone(), now))
            })
            .map(|_| ())
    }

    /// The stored video when a fetched thumbnail can still be placed for it:
    /// it still exists, is still thumbnail-fetchable (not excluded or
    /// errored), and hasn't been given a thumbnail by a download meanwhile.
    fn still_placeable(&self, video: &Video) -> anyhow::Result<Option<Video>> {
        Ok(self.video_repository.find(&video.id)?.filter(|stored| {
            stored.is_thumbnail_fetchable() && stored.thumbnail_filename.is_none()
        }))
    }
}
