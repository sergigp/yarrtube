use crate::domain::playlist::PlaylistPath;
use crate::domain::services::{ThumbnailFetcher, ThumbnailFetcherApi};
use crate::domain::shared::Quality;
use crate::domain::task::{ScheduledTask, Task};
use crate::domain::video::VideoRecordId;
use crate::domain::video::{
    Video, VideoStatus, resolve_output_dir, top_level_entry, video_dir_for_filename,
};
use crate::domain::video_metadata::{build_video_metadata, resolve_sorttitle};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_metadata_repository::YoutubeMetadataRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

/// What the database says a playlist's or channel's output folder should
/// contain: the desired state the folder is reconciled towards.
pub struct DesiredState {
    pub path: PlaylistPath,
    pub quality: Quality,
    pub videos: Vec<Video>,
    /// Each video's position in its playlist, used to resolve its metadata
    /// `sorttitle`. Empty for a channel, whose videos sort by publish date.
    pub sort_positions: HashMap<VideoRecordId, i64>,
}

/// What the YouTube membership sync of the same reconcile pass just changed.
#[derive(Default)]
pub struct MembershipDelta {
    /// Videos added by the sync: their download and thumbnail come from their
    /// own video-added event, not from this reconciler's recoveries.
    pub added_ids: Vec<VideoRecordId>,
    /// The title each renamed video had before the sync: a download started
    /// before the rename is still writing into the folder named after it.
    pub previous_titles: HashMap<VideoRecordId, String>,
}

/// Reconciles our database with our filesystem: brings a playlist's or
/// channel's output folder in line with the videos recorded for it. It is
/// the second step of a reconcile pass, after the membership sync that
/// brings the database in line with YouTube (`PlaylistVideoReconciler`,
/// `ChannelVideoReconciler`), hence "internal": YouTube is never consulted
/// here.
#[derive(Clone)]
pub struct InternalVideoReconciler {
    video_repository: Arc<dyn VideoRepository>,
    youtube_metadata_repository: Arc<dyn YoutubeMetadataRepository>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    task_repository: Arc<dyn TaskRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    thumbnail_fetcher: Arc<ThumbnailFetcher>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl InternalVideoReconciler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        youtube_metadata_repository: Arc<dyn YoutubeMetadataRepository>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        task_repository: Arc<dyn TaskRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        thumbnail_fetcher: Arc<ThumbnailFetcher>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_repository,
            youtube_metadata_repository,
            video_metadata_repository,
            task_repository,
            video_file_repository,
            thumbnail_fetcher,
            clock,
            videos_path: videos_path.into(),
        }
    }
}

pub trait InternalVideoReconcilerApi: Send + Sync {
    /// Brings the folder described by `desired` in line with it, working
    /// from one read of its actual state (files on disk, downloads in
    /// flight): redownloads videos whose file is missing or not mp4, repairs
    /// missing metadata, recovers errored videos past their cooldown,
    /// reschedules stranded downloads, schedules missing thumbnails and
    /// deletes orphaned files. `delta` tells it what the membership sync of
    /// the same pass just added or renamed.
    fn reconcile(&self, desired: &DesiredState, delta: &MembershipDelta) -> anyhow::Result<()>;
}

impl InternalVideoReconcilerApi for InternalVideoReconciler {
    fn reconcile(&self, desired: &DesiredState, delta: &MembershipDelta) -> anyhow::Result<()> {
        let actual = self.read_actual_state(desired, delta)?;

        let redownloaded = self.redownload_broken_files(desired, &actual)?;
        self.generate_missing_metadata(desired, &actual)?;
        let recovered = self.recover_errored_videos(desired, &actual)?;

        // Added videos get their download and thumbnail from their own
        // video-added event, reset ones with their fresh download, so the
        // safety nets below skip them.
        let handled: HashSet<&VideoRecordId> = delta
            .added_ids
            .iter()
            .chain(redownloaded)
            .chain(recovered)
            .collect();
        self.reschedule_stranded_downloads(desired, &actual, &handled)?;
        self.thumbnail_fetcher
            .schedule_missing(&desired.videos, &handled, &actual.output_dir)?;

        self.delete_orphaned_files(&actual)
    }
}

impl InternalVideoReconciler {
    /// Reads the folder's actual state once, so every step works from the
    /// same picture taken before any of them changes something.
    fn read_actual_state(
        &self,
        desired: &DesiredState,
        delta: &MembershipDelta,
    ) -> anyhow::Result<ActualState> {
        let downloads_in_flight = self.video_ids_with_download_in_flight()?;
        let output_dir = resolve_output_dir(&self.videos_path, desired.path.as_str());
        let files = self.video_file_repository.list(&output_dir)?;
        let broken_download_ids = desired
            .videos
            .iter()
            .filter(|v| {
                v.status == VideoStatus::Downloaded && !self.has_healthy_file(v, &output_dir)
            })
            .map(|v| v.id.clone())
            .collect();
        let protected_entries = protected_entries(&desired.videos, &delta.previous_titles);

        Ok(ActualState {
            output_dir,
            files,
            downloads_in_flight,
            broken_download_ids,
            protected_entries,
        })
    }

    fn video_ids_with_download_in_flight(&self) -> anyhow::Result<HashSet<String>> {
        Ok(self
            .task_repository
            .list_non_completed()?
            .iter()
            .filter_map(ScheduledTask::download_video_id)
            .collect())
    }

    /// A video's file is healthy when it exists and is an mp4.
    fn has_healthy_file(&self, video: &Video, output_dir: &Path) -> bool {
        video.filename.as_deref().is_some_and(|filename| {
            self.video_file_repository.file_exists(output_dir, filename)
                && Path::new(filename)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
        })
    }

    /// Resets every `Downloaded` video whose file is missing or not mp4 (a
    /// stale container downloaded before yt-dlp always remuxed to mp4, see
    /// `args_for_quality`) and schedules a fresh download for it; orphan
    /// cleanup removes the stale file once it is redownloaded under a fresh
    /// filename. Returns the ids it reset.
    fn redownload_broken_files<'a>(
        &self,
        desired: &'a DesiredState,
        actual: &ActualState,
    ) -> anyhow::Result<Vec<&'a VideoRecordId>> {
        desired
            .videos
            .iter()
            .filter(|v| actual.is_broken(v))
            .map(|video| {
                warn!(
                    video_id = %video.youtube_id,
                    filename = video.filename.as_deref().unwrap_or(""),
                    "downloaded video's file is missing or not mp4, resetting for redownload"
                );
                self.reset_for_redownload(desired, actual, video)?;
                Ok(&video.id)
            })
            .collect()
    }

    /// Repairs the metadata of every healthy `Downloaded` video that has
    /// none.
    fn generate_missing_metadata(
        &self,
        desired: &DesiredState,
        actual: &ActualState,
    ) -> anyhow::Result<()> {
        for video in desired
            .videos
            .iter()
            .filter(|v| v.status == VideoStatus::Downloaded && !actual.is_broken(v))
        {
            if self.video_metadata_repository.find(&video.id)?.is_none() {
                self.generate_metadata(
                    video,
                    &actual.output_dir,
                    desired.sort_positions.get(&video.id).copied(),
                );
            }
        }

        Ok(())
    }

    /// Regenerates `video`'s metadata (fetch `YoutubeMetadata`, resolve
    /// `sorttitle`, build `VideoMetadata`, save) the same way
    /// `VideoDownloader::download` does at download time. Any failure is
    /// logged and swallowed — a `Downloaded` video's status and file are
    /// never touched by this, and a repeated failure simply tries again on
    /// the next reconcile pass. `sort_position` is `None` for a channel
    /// video, resolving `sorttitle` via publish date instead.
    fn generate_metadata(&self, video: &Video, output_dir: &Path, sort_position: Option<i64>) {
        let Some(filename) = video.filename.as_deref() else {
            return;
        };
        let metadata = match self.youtube_metadata_repository.find(&video.youtube_id) {
            Ok(Some(metadata)) => metadata,
            Ok(None) => {
                warn!(video_id = %video.id, "no YouTube metadata found for video, skipping metadata repair");
                return;
            }
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to fetch YouTube metadata during reconcile, skipping metadata repair");
                return;
            }
        };

        let thumbnail_filename = video
            .thumbnail_filename
            .as_deref()
            .and_then(|f| Path::new(f).file_name())
            .and_then(|f| f.to_str())
            .map(str::to_string);
        let sorttitle = resolve_sorttitle(&metadata.title, metadata.published_at, sort_position);
        let video_metadata = build_video_metadata(
            &video.youtube_id,
            &metadata,
            sorttitle,
            thumbnail_filename,
            self.clock.now(),
        );
        let video_dir = video_dir_for_filename(output_dir, filename);

        if let Err(e) = self
            .video_metadata_repository
            .save(&video.id, &video_metadata, &video_dir)
        {
            warn!(video_id = %video.id, error = %e, "failed to save video metadata during reconcile");
        }
    }

    /// Gives every `Errored` video (one that exhausted its download retries)
    /// another chance once its recovery cooldown is over
    /// (`Video::is_due_for_recovery`), with no limit on how many times a
    /// video may be recovered. Returns the ids it reset.
    fn recover_errored_videos<'a>(
        &self,
        desired: &'a DesiredState,
        actual: &ActualState,
    ) -> anyhow::Result<Vec<&'a VideoRecordId>> {
        let now = self.clock.now();
        desired
            .videos
            .iter()
            .filter(|v| v.is_due_for_recovery(now))
            .map(|video| {
                warn!(
                    video_id = %video.youtube_id,
                    "permanently errored video found during reconcile, resetting for redownload"
                );
                self.reset_for_redownload(desired, actual, video)?;
                Ok(&video.id)
            })
            .collect()
    }

    /// Safety net for a video still waiting to be downloaded that has no
    /// download task pending or running (e.g. lost on a crash): schedules
    /// one again. Skips the `handled` videos, already taken care of.
    fn reschedule_stranded_downloads(
        &self,
        desired: &DesiredState,
        actual: &ActualState,
        handled: &HashSet<&VideoRecordId>,
    ) -> anyhow::Result<()> {
        desired
            .videos
            .iter()
            .filter(|v| {
                !v.is_download_settled()
                    && !actual.downloads_in_flight.contains(v.id.as_str())
                    && !handled.contains(&v.id)
            })
            .try_for_each(|video| {
                warn!(
                    video_id = %video.youtube_id,
                    status = video.status.as_str(),
                    "stranded video found during reconcile, rescheduling its download"
                );
                self.schedule_download(desired, actual, video, self.clock.now())
            })
    }

    /// Deletes every top-level entry of the output folder that doesn't
    /// belong to a stored video.
    fn delete_orphaned_files(&self, actual: &ActualState) -> anyhow::Result<()> {
        for file in &actual.files {
            if actual.protected_entries.contains(file) {
                continue;
            }
            if self
                .video_file_repository
                .delete(&actual.output_dir, file)?
            {
                info!(file, "deleted orphaned file during reconciliation");
            }
        }

        Ok(())
    }

    fn reset_for_redownload(
        &self,
        desired: &DesiredState,
        actual: &ActualState,
        video: &Video,
    ) -> anyhow::Result<()> {
        let now = self.clock.now();
        self.video_repository
            .update(&video.clone().reset_for_redownload(now))?;
        self.schedule_download(desired, actual, video, now)
    }

    fn schedule_download(
        &self,
        desired: &DesiredState,
        actual: &ActualState,
        video: &Video,
        run_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        self.task_repository.schedule(
            &Task::DownloadVideo {
                video_id: video.id.as_str().to_string(),
                quality: desired.quality.as_str().to_string(),
                output_dir: actual.output_dir.to_string_lossy().to_string(),
            },
            run_at,
        )
    }
}

/// The folder as one pass found it, before any step changes it.
struct ActualState {
    output_dir: PathBuf,
    files: Vec<String>,
    /// Ids of every video with a download task pending or running.
    downloads_in_flight: HashSet<String>,
    /// `Downloaded` videos whose file is missing or not mp4.
    broken_download_ids: HashSet<VideoRecordId>,
    /// Top-level entries of `output_dir` that orphan cleanup must keep.
    protected_entries: HashSet<String>,
}

impl ActualState {
    fn is_broken(&self, video: &Video) -> bool {
        self.broken_download_ids.contains(&video.id)
    }
}

/// Top-level entries of the output folder orphan cleanup must keep: every
/// `Downloaded` video's file, every stored video's thumbnail (a
/// `Pending`/`InProgress` video may already have one pre-fetched, see the
/// `video-thumbnails` capability), and the folder an in-flight download or
/// thumbnail fetch is writing into — not recorded yet, so protected by its
/// predicted name.
fn protected_entries(
    videos: &[Video],
    previous_titles: &HashMap<VideoRecordId, String>,
) -> HashSet<String> {
    let downloaded_files = videos
        .iter()
        .filter(|v| v.status == VideoStatus::Downloaded)
        .filter_map(|v| v.filename.clone());
    let thumbnails = videos.iter().filter_map(|v| v.thumbnail_filename.clone());
    let unrecorded_folders = videos.iter().flat_map(|video| {
        video.unrecorded_folder_candidates(previous_titles.get(&video.id).map(String::as_str))
    });

    downloaded_files
        .chain(thumbnails)
        .chain(unrecorded_folders)
        .map(|entry| top_level_entry(&entry).to_string())
        .collect()
}
