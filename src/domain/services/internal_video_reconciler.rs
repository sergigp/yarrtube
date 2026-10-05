use crate::domain::playlist::PlaylistPath;
use crate::domain::services::{
    MetadataGenerator, MetadataGeneratorApi, ThumbnailFetcher, ThumbnailFetcherApi,
};
use crate::domain::shared::Quality;
use crate::domain::task::{ScheduledTask, Task};
use crate::domain::video::VideoRecordId;
use crate::domain::video::{
    Video, VideoStatus, resolve_output_dir, top_level_entry, video_dir_for_filename,
};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
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
    /// The videos the folder holds, in their owner's order. Only their ids:
    /// the videos themselves are read with the folder's actual state, after
    /// the downloads in flight, so a download finishing mid-pass is never
    /// seen as an unsettled video with no download task.
    pub video_ids: Vec<VideoRecordId>,
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
    metadata_generator: Arc<MetadataGenerator>,
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
        metadata_generator: Arc<MetadataGenerator>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        task_repository: Arc<dyn TaskRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        thumbnail_fetcher: Arc<ThumbnailFetcher>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_repository,
            metadata_generator,
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
            .schedule_missing(&actual.videos, &handled, &actual.output_dir)?;

        self.delete_orphaned_files(&actual)
    }
}

impl InternalVideoReconciler {
    /// Reads the folder's actual state once, so every step works from the
    /// same picture taken before any of them changes something. The videos
    /// are read last: a download that completes between these reads is then
    /// seen as `Downloaded`, never as unsettled with no download in flight.
    fn read_actual_state(
        &self,
        desired: &DesiredState,
        delta: &MembershipDelta,
    ) -> anyhow::Result<ActualState> {
        let downloads_in_flight = self.video_ids_with_download_in_flight()?;
        let output_dir = resolve_output_dir(&self.videos_path, desired.path.as_str());
        let files = self.video_file_repository.list(&output_dir)?;
        let videos = self.video_repository.find_many(&desired.video_ids)?;
        let broken_download_ids = videos
            .iter()
            .filter(|v| {
                v.status == VideoStatus::Downloaded && !self.has_healthy_file(v, &output_dir)
            })
            .map(|v| v.id.clone())
            .collect();
        let protected_entries = protected_entries(&videos, &delta.previous_titles);

        Ok(ActualState {
            output_dir,
            files,
            videos,
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
        desired: &DesiredState,
        actual: &'a ActualState,
    ) -> anyhow::Result<Vec<&'a VideoRecordId>> {
        actual
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
        for video in actual
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

    /// Regenerates `video`'s metadata the same way `VideoDownloader` does at
    /// download time, referencing the thumbnail already on disk. Any failure
    /// is logged and swallowed — a `Downloaded` video's status and file are
    /// never touched by this, and a repeated failure simply tries again on
    /// the next reconcile pass.
    fn generate_metadata(&self, video: &Video, output_dir: &Path, sort_position: Option<i64>) {
        let Some(filename) = video.filename.as_deref() else {
            return;
        };
        let thumbnail_filename = video
            .thumbnail_filename
            .as_deref()
            .and_then(|f| Path::new(f).file_name())
            .and_then(|f| f.to_str())
            .map(str::to_string);
        let Some(video_metadata) =
            self.metadata_generator
                .generate(video, sort_position, thumbnail_filename)
        else {
            return;
        };
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
        desired: &DesiredState,
        actual: &'a ActualState,
    ) -> anyhow::Result<Vec<&'a VideoRecordId>> {
        let now = self.clock.now();
        actual
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
        actual
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
    /// The desired videos as stored, read after `downloads_in_flight`.
    videos: Vec<Video>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tasks::log_capture::captured_log_messages;
    use crate::domain::services::MetadataGenerator;
    use crate::domain::task::TaskStatus;
    use crate::domain::video::VideoId;
    use crate::domain::video_metadata::VideoMetadata;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::SqliteVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::repositories::youtube_metadata_repository::{
        FakeYoutubeMetadataRepository, YoutubeMetadata,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use crate::infrastructure::shared::ytdlp::FetchedThumbnail;
    use chrono::Duration;
    use std::sync::Mutex;

    #[test]
    fn it_should_redownload_videos_with_missing_file() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        let video = Video {
            duration_seconds: Some(223),
            ..downloaded_video("My Video.mp4", Some("My Video.jpg"))
        }
        .mark_watched(watched_timestamp());
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                status: VideoStatus::Pending,
                quality: None,
                filename: None,
                thumbnail_filename: None,
                duration_seconds: None,
                updated_at: fixed_timestamp(),
                synced_at: None,
                ..video.clone()
            }]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: "high".to_string(),
                    output_dir: "/videos/my-playlist".to_string(),
                },
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_clear_the_sync_time_of_videos_redownloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        let video = downloaded_video("My Video.mp4", Some("My Video.jpg"));
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                status: VideoStatus::Pending,
                quality: None,
                filename: None,
                thumbnail_filename: None,
                synced_at: None,
                ..video.clone()
            }]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: "high".to_string(),
                    output_dir: "/videos/my-playlist".to_string(),
                },
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_redownload_videos_with_non_mp4_file() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.webm".to_string(),
        ]));
        let video = downloaded_video("My Video.webm", None);
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().reset_for_redownload(fixed_timestamp())]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: "high".to_string(),
                    output_dir: "/videos/my-playlist".to_string(),
                },
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_delete_orphaned_files() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "orphan.mp4".to_string(),
        ]));
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("orphan.mp4")]
        );
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_keep_the_folder_of_a_download_in_progress() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video".to_string(),
        ]));
        let video = my_video().start_download(fixed_timestamp());
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![download_video_task(1, &video.id)]
        );
    }

    #[test]
    fn it_should_keep_the_folder_of_a_thumbnail_fetch_in_progress() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video".to_string(),
        ]));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                download_video_task(1, &video.id),
                fetch_thumbnail_task(2, &video.id)
            ]
        );
    }

    #[test]
    fn it_should_keep_the_suffixed_folder_of_a_download_in_progress() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video [vid1]".to_string(),
        ]));
        let video = my_video().start_download(fixed_timestamp());
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![download_video_task(1, &video.id)]
        );
    }

    #[test]
    fn it_should_keep_the_folder_of_a_download_being_retried() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video".to_string(),
        ]));
        let video = my_video()
            .start_download(fixed_timestamp())
            .mark_errored_retrying(fixed_timestamp());
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                download_video_task(1, &video.id),
                fetch_thumbnail_task(2, &video.id)
            ]
        );
    }

    #[test]
    fn it_should_still_delete_a_folder_no_video_accounts_for() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video".to_string(),
            "Other".to_string(),
        ]));
        let video = my_video().start_download(fixed_timestamp());
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("Other")]
        );
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![download_video_task(1, &video.id)]
        );
    }

    #[test]
    fn it_should_retry_permanently_errored_videos() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        let video = my_video()
            .start_download(a_day_ago())
            .mark_errored(a_day_ago());
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().reset_for_redownload(fixed_timestamp())]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: "high".to_string(),
                    output_dir: "/videos/my-playlist".to_string(),
                },
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_retry_a_video_that_failed_again() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        let failed_again = my_video()
            .start_download(a_day_ago())
            .mark_errored(a_day_ago())
            .reset_for_redownload(a_day_ago())
            .start_download(a_day_ago())
            .mark_errored(a_day_ago());
        video_repository.save(&failed_again).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(failed_again.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![failed_again.clone().reset_for_redownload(fixed_timestamp())]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![download_video_task(1, &failed_again.id)]
        );
    }

    #[test]
    fn it_should_not_retry_videos_errored_within_the_last_day() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        let errored_at = fixed_timestamp() - Duration::hours(23);
        let video = my_video()
            .start_download(errored_at)
            .mark_errored(errored_at);
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_reschedule_the_download_of_an_errored_retrying_video_with_no_download_task() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        let video = my_video()
            .start_download(a_day_ago())
            .mark_errored_retrying(a_day_ago());
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                download_video_task(1, &video.id),
                fetch_thumbnail_task(2, &video.id)
            ]
        );
    }

    #[test]
    fn it_should_reschedule_the_download_of_an_in_progress_video_with_no_download_task() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        let video = my_video().start_download(a_day_ago());
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![download_video_task(1, &video.id)]
        );
    }

    #[test]
    fn it_should_reschedule_the_download_of_a_pending_video_with_no_download_task() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                download_video_task(1, &video.id),
                fetch_thumbnail_task(2, &video.id)
            ]
        );
    }

    #[test]
    fn it_should_not_reschedule_a_non_terminal_video_whose_download_is_queued() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        let video = my_video()
            .start_download(a_day_ago())
            .mark_errored_retrying(a_day_ago());
        video_repository.save(&video).unwrap();
        task_repository
            .schedule(
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: "high".to_string(),
                    output_dir: "/videos/my-playlist".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let mut result = Err(String::new());
        let logs = captured_log_messages(|| {
            result = reconcile(&reconciler, &desired, &MembershipDelta::default())
        });

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                download_video_task(1, &video.id),
                fetch_thumbnail_task(2, &video.id)
            ]
        );
        assert!(
            !logs
                .iter()
                .any(|message| message.contains("stranded video found during reconcile"))
        );
    }

    #[test]
    fn it_should_not_reschedule_a_downloaded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_file_exists(true));
        let video = downloaded_video("My Video.mp4", Some("My Video.jpg"));
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_never_recover_an_excluded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        let long_ago = fixed_timestamp() - Duration::days(400);
        let video = my_video().start_download(long_ago).mark_excluded(long_ago);
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_keep_matching_files() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.mp4".to_string(),
        ]));
        let video = downloaded_video("My Video.mp4", None);
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![fetch_thumbnail_task(1, &video.id)]
        );
    }

    #[test]
    fn it_should_keep_matching_thumbnails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.mp4".to_string(),
            "My Video.jpg".to_string(),
        ]));
        let video = downloaded_video("My Video.mp4", Some("My Video.jpg"));
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_delete_stray_thumbnails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.mp4".to_string(),
            "stray.jpg".to_string(),
        ]));
        let video = downloaded_video("My Video.mp4", None);
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("stray.jpg")]
        );
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![fetch_thumbnail_task(1, &video.id)]
        );
    }

    #[test]
    fn it_should_keep_videos_stored_in_their_own_folder() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository {
            list_result: Mutex::new(Some(Ok(vec!["My Video".to_string()]))),
            file_exists_result: Mutex::new(Some(true)),
            ..Default::default()
        });
        let video = downloaded_video("My Video/My Video.mp4", None);
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![fetch_thumbnail_task(1, &video.id)]
        );
    }

    #[test]
    fn it_should_keep_thumbnails_of_pending_videos() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video".to_string(),
        ]));
        let video = my_video().with_thumbnail("My Video/My Video.jpg", fixed_timestamp());
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![download_video_task(1, &video.id)]
        );
    }

    #[test]
    fn it_should_generate_missing_metadata() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let videos_root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(videos_root.path().join("my-playlist/My Video")).unwrap();
        let video = downloaded_video("My Video/My Video.mp4", None);
        video_repository.save(&video).unwrap();
        let reconciler = InternalVideoReconciler::new(
            video_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository {
                    metadata: Some(youtube_metadata("My Video")),
                }),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            video_metadata_repository.clone(),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                Arc::new(FakeVideoDownloaderRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            videos_root.path().to_string_lossy(),
        );
        let desired = desired_state(vec![(video.clone(), 3)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(VideoMetadata::new(
                "My Video",
                "A description",
                "My Channel",
                "My Channel",
                fixed_timestamp(),
                None,
                Vec::new(),
                "vid1",
                None,
                "0003 My Video",
                fixed_timestamp(),
            ))
        );
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchThumbnail {
                    video_id: video.id.as_str().to_string(),
                    output_dir: videos_root
                        .path()
                        .join("my-playlist")
                        .to_string_lossy()
                        .to_string(),
                },
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_generate_missing_metadata_sorted_by_publish_date_without_a_position() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let videos_root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(videos_root.path().join("my-playlist/My Video")).unwrap();
        let video = downloaded_video("My Video/My Video.mp4", None);
        video_repository.save(&video).unwrap();
        let reconciler = InternalVideoReconciler::new(
            video_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository {
                    metadata: Some(youtube_metadata("My Video")),
                }),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            video_metadata_repository.clone(),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                Arc::new(FakeVideoDownloaderRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            videos_root.path().to_string_lossy(),
        );
        let desired = DesiredState {
            sort_positions: HashMap::new(),
            ..desired_state(vec![(video.clone(), 3)])
        };

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(VideoMetadata::new(
                "My Video",
                "A description",
                "My Channel",
                "My Channel",
                fixed_timestamp(),
                None,
                Vec::new(),
                "vid1",
                None,
                "20231114 My Video",
                fixed_timestamp(),
            ))
        );
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchThumbnail {
                    video_id: video.id.as_str().to_string(),
                    output_dir: videos_root
                        .path()
                        .join("my-playlist")
                        .to_string_lossy()
                        .to_string(),
                },
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_keep_existing_metadata() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let videos_root = tempfile::tempdir().unwrap();
        let video_dir = videos_root.path().join("my-playlist/My Video");
        std::fs::create_dir_all(&video_dir).unwrap();
        let video = downloaded_video("My Video/My Video.mp4", None);
        video_repository.save(&video).unwrap();
        let existing_metadata = VideoMetadata::new(
            "Stale Title",
            "Stale plot",
            "Stale Channel",
            "Stale Channel",
            DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            None,
            Vec::new(),
            "vid1",
            None,
            "0000 Stale Title",
            DateTime::UNIX_EPOCH,
        );
        video_metadata_repository
            .save(&video.id, &existing_metadata, &video_dir)
            .unwrap();
        let reconciler = InternalVideoReconciler::new(
            video_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository {
                    metadata: Some(youtube_metadata("Fresh Title")),
                }),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            video_metadata_repository.clone(),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                Arc::new(FakeVideoDownloaderRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            videos_root.path().to_string_lossy(),
        );
        let desired = desired_state(vec![(video.clone(), 3)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(existing_metadata)
        );
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchThumbnail {
                    video_id: video.id.as_str().to_string(),
                    output_dir: videos_root
                        .path()
                        .join("my-playlist")
                        .to_string_lossy()
                        .to_string(),
                },
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_not_generate_metadata_for_a_video_being_redownloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let videos_root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(videos_root.path().join("my-playlist/My Video")).unwrap();
        let video = downloaded_video("My Video/My Video.mp4", None);
        video_repository.save(&video).unwrap();
        let reconciler = InternalVideoReconciler::new(
            video_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository {
                    metadata: Some(youtube_metadata("My Video")),
                }),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            video_metadata_repository.clone(),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_file_exists(false)),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                Arc::new(FakeVideoDownloaderRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            videos_root.path().to_string_lossy(),
        );
        let desired = desired_state(vec![(video.clone(), 3)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(video_metadata_repository.find(&video.id).unwrap(), None);
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().reset_for_redownload(fixed_timestamp())]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: "high".to_string(),
                    output_dir: videos_root
                        .path()
                        .join("my-playlist")
                        .to_string_lossy()
                        .to_string(),
                },
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_schedule_a_thumbnail_fetch_for_a_video_missing_one() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_thumbnail_result(Some(fetched_thumbnail())),
        );
        let video = my_video();
        video_repository.save(&video).unwrap();
        let reconciler = InternalVideoReconciler::new(
            video_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                download_video_task(1, &video.id),
                fetch_thumbnail_task(2, &video.id)
            ]
        );
    }

    #[test]
    fn it_should_not_schedule_a_second_thumbnail_fetch_if_one_is_queued() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_downloader_repository = Arc::new(FakeVideoDownloaderRepository::default());
        let video = my_video();
        video_repository.save(&video).unwrap();
        task_repository
            .schedule(
                &Task::FetchThumbnail {
                    video_id: video.id.as_str().to_string(),
                    output_dir: "/videos/my-playlist".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let reconciler = InternalVideoReconciler::new(
            video_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                fetch_thumbnail_task(1, &video.id),
                download_video_task(2, &video.id)
            ]
        );
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
    }

    #[test]
    fn it_should_not_schedule_a_thumbnail_fetch_for_a_video_being_downloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_downloader_repository = Arc::new(FakeVideoDownloaderRepository::default());
        let video = my_video().start_download(fixed_timestamp());
        video_repository.save(&video).unwrap();
        task_repository
            .schedule(
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: "high".to_string(),
                    output_dir: "/videos/my-playlist".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let reconciler = InternalVideoReconciler::new(
            video_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![]
        );
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![download_video_task(1, &video.id)]
        );
    }

    #[test]
    fn it_should_not_schedule_a_thumbnail_fetch_for_an_excluded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = my_video().mark_excluded(a_day_ago());
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_schedule_a_thumbnail_fetch_for_an_errored_video_not_due_for_recovery() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let errored_at = fixed_timestamp() - Duration::hours(23);
        let video = my_video()
            .start_download(errored_at)
            .mark_errored(errored_at);
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_refetch_existing_thumbnails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_downloader_repository = Arc::new(FakeVideoDownloaderRepository::default());
        let video = downloaded_video("My Video/My Video.mp4", Some("My Video/My Video.jpg"));
        video_repository.save(&video).unwrap();
        let reconciler = InternalVideoReconciler::new(
            video_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![]
        );
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_schedule_a_thumbnail_fetch_for_a_video_being_redownloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_thumbnail_result(Some(fetched_thumbnail())),
        );
        let video = downloaded_video("My Video/My Video.mp4", None);
        video_repository.save(&video).unwrap();
        let reconciler = InternalVideoReconciler::new(
            video_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(Vec::new())),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        );
        let desired = desired_state(vec![(video.clone(), 0)]);

        let result = reconcile(&reconciler, &desired, &MembershipDelta::default());

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![]
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().reset_for_redownload(fixed_timestamp())]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: "high".to_string(),
                    output_dir: "/videos/my-playlist".to_string(),
                },
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_keep_the_folder_of_a_download_in_progress_if_video_renamed() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video".to_string(),
        ]));
        let renamed = Video {
            title: "Renamed".to_string(),
            ..my_video().start_download(fixed_timestamp())
        };
        video_repository.save(&renamed).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(renamed.clone(), 0)]);
        let delta = MembershipDelta {
            previous_titles: HashMap::from([(renamed.id.clone(), "My Video".to_string())]),
            ..MembershipDelta::default()
        };

        let result = reconcile(&reconciler, &desired, &delta);

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![renamed.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![download_video_task(1, &renamed.id)]
        );
    }

    #[test]
    fn it_should_not_schedule_a_download_or_thumbnail_fetch_for_a_video_added_in_the_same_pass() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let added = my_video();
        video_repository.save(&added).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![(added.clone(), 0)]);
        let delta = MembershipDelta {
            added_ids: vec![added.id.clone()],
            ..MembershipDelta::default()
        };

        let result = reconcile(&reconciler, &desired, &delta);

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![added]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    /// Builds the reconciler around the ports these tests observe; the
    /// metadata and thumbnail ports are filled in with inert fakes.
    fn internal_video_reconciler(
        db: &TestDatabase,
        video_repository: Arc<SqliteVideoRepository>,
        task_repository: Arc<SqliteTaskRepository>,
        video_file_repository: Arc<FakeVideoFileRepository>,
    ) -> InternalVideoReconciler {
        InternalVideoReconciler::new(
            video_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            task_repository.clone(),
            video_file_repository,
            Arc::new(ThumbnailFetcher::new(
                video_repository,
                Arc::new(FakeVideoDownloaderRepository::default()),
                task_repository,
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        )
    }

    /// The `my-playlist` folder holding `videos`, each at its playlist
    /// position, ordered by position as the database lists them.
    fn desired_state(videos: Vec<(Video, i64)>) -> DesiredState {
        let mut videos = videos;
        videos.sort_by_key(|(_, position)| *position);
        DesiredState {
            path: PlaylistPath::new("my-playlist").unwrap(),
            quality: Quality::High,
            sort_positions: videos
                .iter()
                .map(|(video, position)| (video.id.clone(), *position))
                .collect(),
            video_ids: videos.into_iter().map(|(video, _)| video.id).collect(),
        }
    }

    fn my_video() -> Video {
        Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp())
    }

    fn downloaded_video(filename: &str, thumbnail_filename: Option<&str>) -> Video {
        my_video()
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                filename,
                thumbnail_filename.map(str::to_string),
                None,
                fixed_timestamp(),
            )
    }

    fn youtube_metadata(title: &str) -> YoutubeMetadata {
        YoutubeMetadata {
            title: title.to_string(),
            description: "A description".to_string(),
            channel_title: "My Channel".to_string(),
            published_at: fixed_timestamp(),
            tags: Vec::new(),
            category_id: None,
        }
    }

    /// The thumbnail the fake downloader reports for `my_video()`.
    fn fetched_thumbnail() -> FetchedThumbnail {
        FetchedThumbnail {
            folder: "My Video".to_string(),
            filename: "My Video.jpg".to_string(),
        }
    }

    /// The `(output_dir, entry)` pair the fake records for one delete call.
    fn deleted(entry: &str) -> (PathBuf, String) {
        (PathBuf::from("/videos/my-playlist"), entry.to_string())
    }

    fn download_video_task(id: i64, video_id: &VideoRecordId) -> ScheduledTask {
        pending_task(
            id,
            &Task::DownloadVideo {
                video_id: video_id.as_str().to_string(),
                quality: "high".to_string(),
                output_dir: "/videos/my-playlist".to_string(),
            },
            fixed_timestamp(),
        )
    }

    fn fetch_thumbnail_task(id: i64, video_id: &VideoRecordId) -> ScheduledTask {
        pending_task(
            id,
            &Task::FetchThumbnail {
                video_id: video_id.as_str().to_string(),
                output_dir: "/videos/my-playlist".to_string(),
            },
            fixed_timestamp(),
        )
    }

    fn pending_task(id: i64, task: &Task, run_at: DateTime<Utc>) -> ScheduledTask {
        ScheduledTask {
            id,
            task_type: task.task_type().to_string(),
            payload: task.payload().to_string(),
            status: TaskStatus::Pending,
            retries: 0,
            run_at,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn watched_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_800_000_000, 0).unwrap()
    }

    fn a_day_ago() -> DateTime<Utc> {
        fixed_timestamp() - Duration::hours(24)
    }

    fn reconcile(
        reconciler: &InternalVideoReconciler,
        desired: &DesiredState,
        delta: &MembershipDelta,
    ) -> Result<(), String> {
        reconciler
            .reconcile(desired, delta)
            .map_err(|e| e.to_string())
    }
}
