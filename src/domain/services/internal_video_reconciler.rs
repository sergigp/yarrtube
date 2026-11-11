use crate::domain::playlist::PlaylistPath;
use crate::domain::task::Task;
use crate::domain::video::VideoId;
use crate::domain::video::{
    EpisodePaths, SHOW_NFO_FILENAME, SHOW_POSTER_FILENAME, Video, is_episode_path,
    is_season_folder, nfo_path_for, owning_entry, resolve_output_dir,
};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

/// The extensions of the files a download or thumbnail fetch moves into
/// place under an episode's base name, see `EpisodePaths`.
const EPISODE_FILE_EXTENSIONS: [&str; 3] = ["mp4", "jpg", "nfo"];

/// What the database says a playlist's or channel's output folder should
/// contain: the desired state the folder is reconciled towards.
pub struct DesiredState {
    pub path: PlaylistPath,
    /// The videos the folder holds, in their owner's order.
    pub video_ids: Vec<VideoId>,
}

/// What bringing a folder in line with the database did, for the pass's
/// summary line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderReconciliation {
    /// Video syncs requested, one per desired video (each scheduled unless
    /// one is already pending).
    pub syncs_requested: usize,
    pub orphans_deleted: usize,
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
    video_file_repository: Arc<dyn VideoFileRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl InternalVideoReconciler {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_repository,
            video_file_repository,
            task_repository,
            clock,
            videos_path: videos_path.into(),
        }
    }
}

pub trait InternalVideoReconcilerApi: Send + Sync {
    /// Schedules the video sync of every video of `desired` (a `sync_video`
    /// task, run under the video's task key), then deletes orphaned files.
    /// The stale file of a video its sync resets is swept by the next pass.
    fn reconcile(&self, desired: &DesiredState) -> anyhow::Result<FolderReconciliation>;
}

impl InternalVideoReconcilerApi for InternalVideoReconciler {
    fn reconcile(&self, desired: &DesiredState) -> anyhow::Result<FolderReconciliation> {
        let syncs_requested = self.schedule_syncs(desired)?;
        let actual = self.read_actual_state(desired)?;
        let orphans_deleted = self.delete_orphaned_files(&actual)?;
        Ok(FolderReconciliation {
            syncs_requested,
            orphans_deleted,
        })
    }
}

impl InternalVideoReconciler {
    /// Reads the folder's actual state: the files on disk, then the videos
    /// they should belong to.
    fn read_actual_state(&self, desired: &DesiredState) -> anyhow::Result<ActualState> {
        let output_dir = resolve_output_dir(&self.videos_path, desired.path.as_str());
        let files = self.video_file_repository.list(&output_dir)?;
        let videos = self.video_repository.find_many(&desired.video_ids)?;

        Ok(ActualState {
            output_dir,
            files,
            protected_entries: protected_entries(&videos),
            numbered_episode_paths: numbered_episode_paths(&videos),
            subtitle_pending_paths: subtitle_pending_paths(&videos),
        })
    }

    /// Schedules the sync of every video of `desired`, including those the
    /// same pass just added: the safety net for a step lost on a crash or
    /// never scheduled. A sync already pending is not scheduled twice. How
    /// many were requested.
    fn schedule_syncs(&self, desired: &DesiredState) -> anyhow::Result<usize> {
        desired.video_ids.iter().try_for_each(|id| {
            self.task_repository.schedule(
                &Task::SyncVideo {
                    video_id: id.as_str().to_string(),
                },
                self.clock.now(),
            )
        })?;
        Ok(desired.video_ids.len())
    }

    /// Deletes every entry of the output folder that is neither a show file,
    /// a season folder, nor a file of a stored video; how many it deleted.
    fn delete_orphaned_files(&self, actual: &ActualState) -> anyhow::Result<usize> {
        actual
            .files
            .iter()
            .filter(|file| !actual.is_protected(file))
            .try_fold(0, |deleted, file| {
                let removed = self
                    .video_file_repository
                    .delete(&actual.output_dir, file)?;
                if removed {
                    info!(file, "deleted orphaned file during reconciliation");
                }
                Ok(deleted + usize::from(removed))
            })
    }
}

/// The folder as one pass found it.
struct ActualState {
    output_dir: PathBuf,
    files: Vec<String>,
    /// Entries of `output_dir` that orphan cleanup must keep.
    protected_entries: HashSet<String>,
    /// `Season <s>/<base name>` of every numbered video: the media, thumbnail
    /// or NFO a download or thumbnail fetch already moved into place under it
    /// is kept, even before the video records it. A stale container (`.mkv`,
    /// `.webm`) under it is not, so a reset video's old file is swept.
    numbered_episode_paths: HashSet<String>,
    /// The recorded media file's stem of every numbered video whose
    /// subtitles are still Missing: a subtitle fetch may already have moved
    /// its `<stem>.<lang>.srt` into place without recording it yet.
    subtitle_pending_paths: HashSet<String>,
}

impl ActualState {
    fn is_protected(&self, entry: &str) -> bool {
        self.protected_entries.contains(entry)
            || is_season_folder(entry)
            || self.is_numbered_episode_file(entry)
            || self.subtitle_pending_paths.iter().any(|stem| {
                entry
                    .strip_prefix(stem.as_str())
                    .is_some_and(|rest| rest.starts_with('.'))
            })
    }

    fn is_numbered_episode_file(&self, entry: &str) -> bool {
        let path = Path::new(entry);
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| EPISODE_FILE_EXTENSIONS.contains(&ext))
            && self
                .numbered_episode_paths
                .contains(path.with_extension("").to_string_lossy().as_ref())
    }
}

/// Entries of the output folder orphan cleanup must keep: the show files,
/// every `Downloaded` video's file and episode NFO, every stored video's
/// thumbnail (a `Pending`/`InProgress` video may already have one
/// pre-fetched, see the `video-thumbnails` capability) and every stored
/// video's subtitle file. An episode file is kept by its path in its season
/// folder, a legacy video by its top-level folder or flat file.
fn protected_entries(videos: &[Video]) -> HashSet<String> {
    let downloaded_files: Vec<&str> = videos
        .iter()
        .filter(|v| v.download.is_downloaded())
        .filter_map(|v| v.download.filename())
        .collect();
    let thumbnails = videos
        .iter()
        .filter_map(|v| v.thumbnail.filename.as_deref());
    let subtitles = videos
        .iter()
        .filter_map(|v| v.subtitles.filename.as_deref());
    let episode_nfos = downloaded_files
        .iter()
        .filter(|filename| is_episode_path(filename))
        .map(|filename| nfo_path_for(filename));
    let recorded = downloaded_files
        .iter()
        .copied()
        .chain(thumbnails)
        .chain(subtitles)
        .map(|path| owning_entry(path).to_string());
    [SHOW_NFO_FILENAME, SHOW_POSTER_FILENAME]
        .map(str::to_string)
        .into_iter()
        .chain(recorded)
        .chain(episode_nfos)
        .collect()
}

fn numbered_episode_paths(videos: &[Video]) -> HashSet<String> {
    videos
        .iter()
        .filter_map(|v| {
            v.episode_number
                .map(|number| EpisodePaths::new(&number, &v.title).stem())
        })
        .collect()
}

fn subtitle_pending_paths(videos: &[Video]) -> HashSet<String> {
    videos
        .iter()
        .filter(|v| {
            v.subtitles.is_unsettled()
                && v.episode_number.is_some()
                && !v.download.is_in_legacy_layout()
        })
        .filter_map(|v| v.download.filename())
        .map(|filename| EpisodePaths::from_media(filename).stem())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::shared::Quality;
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::domain::video::YoutubeVideoId;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;

    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };

    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;

    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;

    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

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
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![]);

        let result = reconcile(&reconciler, &desired);

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("orphan.mp4")]
        );
        assert_eq!(video_repository.list().unwrap(), vec![]);
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
        video_repository.insert(&video).unwrap();
        let reconciler = internal_video_reconciler(
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![video.clone()]);

        let result = reconcile(&reconciler, &desired);

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![sync_video_task(1, &video.id)]
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
        video_repository.insert(&video).unwrap();
        let reconciler = internal_video_reconciler(
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![video.clone()]);

        let result = reconcile(&reconciler, &desired);

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![sync_video_task(1, &video.id)]
        );
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
        video_repository.insert(&video).unwrap();
        let reconciler = internal_video_reconciler(
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![video.clone()]);

        let result = reconcile(&reconciler, &desired);

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("stray.jpg")]
        );
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![sync_video_task(1, &video.id)]
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
        video_repository.insert(&video).unwrap();
        let reconciler = internal_video_reconciler(
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![video.clone()]);

        let result = reconcile(&reconciler, &desired);

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![sync_video_task(1, &video.id)]
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
        video_repository.insert(&video).unwrap();
        let reconciler = internal_video_reconciler(
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![video.clone()]);

        let result = reconcile(&reconciler, &desired);

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![sync_video_task(1, &video.id)]
        );
    }

    /// Builds the reconciler around the ports these tests observe.
    fn internal_video_reconciler(
        video_repository: Arc<SqliteVideoRepository>,
        task_repository: Arc<SqliteTaskRepository>,
        video_file_repository: Arc<FakeVideoFileRepository>,
    ) -> InternalVideoReconciler {
        InternalVideoReconciler::new(
            video_repository.clone(),
            video_file_repository.clone(),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        )
    }

    /// The `my-playlist` folder holding `videos`.
    fn desired_state(videos: Vec<Video>) -> DesiredState {
        DesiredState {
            path: PlaylistPath::new("my-playlist").unwrap(),
            video_ids: videos.into_iter().map(|video| video.id).collect(),
        }
    }

    fn my_video() -> Video {
        Video::create(
            YoutubeVideoId::new("vid1").unwrap(),
            "My Video",
            fixed_timestamp(),
        )
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

    /// The `(output_dir, entry)` pair the fake records for one delete call.
    fn deleted(entry: &str) -> (PathBuf, String) {
        (PathBuf::from("/videos/my-playlist"), entry.to_string())
    }

    fn sync_video_task(id: i64, video_id: &VideoId) -> ScheduledTask {
        ScheduledTask {
            id,
            task: Task::SyncVideo {
                video_id: video_id.as_str().to_string(),
            },
            status: TaskStatus::Pending,
            retries: 0,
            run_at: fixed_timestamp(),
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn reconcile(
        reconciler: &InternalVideoReconciler,
        desired: &DesiredState,
    ) -> Result<(), String> {
        reconciler
            .reconcile(desired)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
