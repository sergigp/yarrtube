use crate::domain::task::Task;
use crate::domain::video::Video;
use crate::domain::video::VideoRecordId;
use crate::domain::video::VideoStatus;
use crate::domain::video::top_level_entry;
use crate::domain::video::video_filename::VideoFilename;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::{
    FetchedThumbnail, VideoDownloaderRepository,
};
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tracing::warn;

/// Best-effort thumbnail fetch, independent of and ahead of a video's full
/// download — see the `video-thumbnails` capability. Called from every
/// video-creation site and from each reconciler's missing-thumbnail
/// recovery pass; never returns an error to its caller, so no call site
/// needs its own try/catch-and-ignore boilerplate.
#[derive(Clone)]
pub struct ThumbnailFetcher {
    video_repository: Arc<dyn VideoRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl ThumbnailFetcher {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_repository,
            video_downloader_repository,
            task_repository,
            clock,
        }
    }
}

pub trait ThumbnailFetcherApi: Send + Sync {
    /// No-ops if `video` already has a recorded thumbnail. Otherwise fetches
    /// one into `output_dir` and persists it via `Video::with_thumbnail` on
    /// success; any failure (a clean "no thumbnail available" outcome, or a
    /// systemic error) is logged and swallowed, leaving `video` untouched.
    /// A `Downloaded` video already has its own folder recorded via
    /// `filename` (e.g. a missing-thumbnail recovery pass running against a
    /// video whose full download already ran) — that folder is reused
    /// verbatim instead of resolving a fresh, collision-suffixed one.
    fn fetch(&self, video: &Video, output_dir: &Path);

    /// Missing-thumbnail recovery pass over `videos`, shared by both
    /// reconcilers' `reconcile_filesystem`. Skips a video with no
    /// thumbnail if it's in `skip_ids` (just reset for redownload this same
    /// pass — see `fetch`'s own reasons this must not run for it) or if its
    /// real download is currently `InProgress` (a concurrent `DownloadVideo`
    /// task owns its not-yet-recorded output folder; fetching now would
    /// resolve `existing_folder` to `None` and collide with it, spawning a
    /// stray sibling folder that then gets permanently protected from the
    /// orphan sweep).
    fn fetch_missing(
        &self,
        videos: &[Video],
        skip_ids: &HashSet<&VideoRecordId>,
        output_dir: &Path,
    );

    /// Schedules a `FetchThumbnail` for each video with no thumbnail, not in
    /// `skip_ids` and not `InProgress`. Dedupe is left to
    /// `TaskRepository::schedule`.
    fn schedule_missing(
        &self,
        videos: &[Video],
        skip_ids: &HashSet<&VideoRecordId>,
        output_dir: &Path,
    ) -> anyhow::Result<()>;
}

impl ThumbnailFetcherApi for ThumbnailFetcher {
    fn fetch(&self, video: &Video, output_dir: &Path) {
        if video.thumbnail_filename.is_some() {
            return;
        }

        match self.fetch_thumbnail(video, output_dir) {
            Ok(Some(fetched)) => self.record_thumbnail(video, fetched),
            Ok(None) => warn!(video_id = %video.id, "no thumbnail available for video"),
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to fetch video thumbnail");
            }
        }
    }

    fn fetch_missing(
        &self,
        videos: &[Video],
        skip_ids: &HashSet<&VideoRecordId>,
        output_dir: &Path,
    ) {
        for video in videos.iter().filter(|v| {
            v.thumbnail_filename.is_none()
                && !skip_ids.contains(&v.id)
                && v.status != VideoStatus::InProgress
        }) {
            self.fetch(video, output_dir);
        }
    }

    fn schedule_missing(
        &self,
        videos: &[Video],
        skip_ids: &HashSet<&VideoRecordId>,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        videos
            .iter()
            .filter(|v| v.thumbnail_filename.is_none() && !skip_ids.contains(&v.id))
            .try_for_each(|video| self.schedule_fetch(video, output_dir))
    }
}

impl ThumbnailFetcher {
    /// Reuses `video`'s already-recorded folder, if any.
    fn fetch_thumbnail(
        &self,
        video: &Video,
        output_dir: &Path,
    ) -> anyhow::Result<Option<FetchedThumbnail>> {
        let existing_folder = video.filename.as_deref().map(top_level_entry);
        let filename = VideoFilename::from_title(&video.title);
        self.video_downloader_repository.fetch_thumbnail(
            &video.youtube_id.to_url(),
            filename.as_str(),
            video.youtube_id.as_str(),
            output_dir,
            existing_folder,
        )
    }

    fn schedule_fetch(&self, video: &Video, output_dir: &Path) -> anyhow::Result<()> {
        self.task_repository.schedule(
            &Task::FetchThumbnail {
                video_id: video.id.as_str().to_string(),
                output_dir: output_dir.to_string_lossy().to_string(),
            },
            self.clock.now(),
        )
    }

    fn record_thumbnail(&self, video: &Video, fetched: FetchedThumbnail) {
        let thumbnail_filename = format!("{}/{}", fetched.folder, fetched.filename);
        let updated = video
            .clone()
            .with_thumbnail(thumbnail_filename, self.clock.now());
        if let Err(e) = self.video_repository.update(&updated) {
            warn!(video_id = %video.id, error = %e, "failed to persist fetched thumbnail");
        }
    }
}
