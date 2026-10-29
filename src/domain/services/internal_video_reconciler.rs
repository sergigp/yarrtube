use crate::domain::playlist::PlaylistPath;
use crate::domain::services::{VideoReconciler, VideoReconcilerApi};
use crate::domain::video::VideoRecordId;
use crate::domain::video::{
    EpisodePaths, SHOW_NFO_FILENAME, SHOW_POSTER_FILENAME, SubtitleState, Video, VideoStatus,
    is_episode_path, is_season_folder, nfo_path_for, owning_entry, resolve_output_dir,
};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

/// What the database says a playlist's or channel's output folder should
/// contain: the desired state the folder is reconciled towards.
pub struct DesiredState {
    pub path: PlaylistPath,
    /// The videos the folder holds, in their owner's order.
    pub video_ids: Vec<VideoRecordId>,
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
    video_reconciler: VideoReconciler,
    videos_path: String,
}

impl InternalVideoReconciler {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        video_reconciler: VideoReconciler,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_repository,
            video_file_repository,
            video_reconciler,
            videos_path: videos_path.into(),
        }
    }
}

pub trait InternalVideoReconcilerApi: Send + Sync {
    /// Reconciles every video of `desired` (`VideoReconciler::reconcile`,
    /// a failing video is logged and skipped), then deletes orphaned files.
    fn reconcile(&self, desired: &DesiredState) -> anyhow::Result<()>;
}

impl InternalVideoReconcilerApi for InternalVideoReconciler {
    fn reconcile(&self, desired: &DesiredState) -> anyhow::Result<()> {
        let actual = self.read_actual_state(desired)?;
        self.sync_videos(desired);
        self.delete_orphaned_files(&actual)
    }
}

impl InternalVideoReconciler {
    /// Reads the folder's actual state: the files on disk and the videos
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

    /// Syncs every video of `desired`, including those the same pass just
    /// added: the safety net for a step lost on a crash or never scheduled.
    /// A video whose sync fails is logged and skipped, so the others are
    /// still synced.
    fn sync_videos(&self, desired: &DesiredState) {
        desired.video_ids.iter().for_each(|id| {
            if let Err(e) = self.video_reconciler.reconcile(id) {
                warn!(video_id = %id, error = %e, "failed to sync video during reconcile");
            }
        });
    }

    /// Deletes every entry of the output folder that is neither a show file,
    /// a season folder, nor a file of a stored video.
    fn delete_orphaned_files(&self, actual: &ActualState) -> anyhow::Result<()> {
        for file in &actual.files {
            if actual.is_protected(file) {
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
}

/// The folder as one pass found it.
struct ActualState {
    output_dir: PathBuf,
    files: Vec<String>,
    /// Entries of `output_dir` that orphan cleanup must keep.
    protected_entries: HashSet<String>,
    /// `Season <s>/<base name>` of every numbered video: whatever a download
    /// or thumbnail fetch already moved into place under it is kept, even
    /// before the video records it.
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
            || self.numbered_episode_paths.contains(
                Path::new(entry)
                    .with_extension("")
                    .to_string_lossy()
                    .as_ref(),
            )
            || self.subtitle_pending_paths.iter().any(|stem| {
                entry
                    .strip_prefix(stem.as_str())
                    .is_some_and(|rest| rest.starts_with('.'))
            })
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
        .filter(|v| v.status == VideoStatus::Downloaded)
        .filter_map(|v| v.filename.as_deref())
        .collect();
    let thumbnails = videos
        .iter()
        .filter_map(|v| v.thumbnail_filename.as_deref());
    let subtitles = videos.iter().filter_map(|v| v.subtitle_filename.as_deref());
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
            v.subtitle_state == SubtitleState::Missing
                && v.episode_number.is_some()
                && !v.is_in_legacy_layout()
        })
        .filter_map(|v| v.filename.as_deref())
        .map(|filename| EpisodePaths::from_media(filename).stem())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::services::MetadataGenerator;
    use crate::domain::services::VideoCategoryAssigner;
    use crate::domain::services::VideoRelocator;
    use crate::domain::shared::Quality;
    use crate::domain::video::VideoId;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_category_repository::SqliteCategoryRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::SqliteChannelRepository;
    use crate::infrastructure::repositories::sqlite_channel_video_repository::SqliteChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_categories_repository::SqliteVideoCategoriesRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::SqliteVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::shared::category_lock::CategoryLock;
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::sqlite_connection::Database;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use crate::infrastructure::shared::video_lock::VideoLock;
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
            &db,
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
        video_repository.save(&video).unwrap();
        let reconciler = internal_video_reconciler(
            &db,
            video_repository.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
        );
        let desired = desired_state(vec![video.clone()]);

        let result = reconcile(&reconciler, &desired);

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
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
        let desired = desired_state(vec![video.clone()]);

        let result = reconcile(&reconciler, &desired);

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
        let desired = desired_state(vec![video.clone()]);

        let result = reconcile(&reconciler, &desired);

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("stray.jpg")]
        );
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
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
        let desired = desired_state(vec![video.clone()]);

        let result = reconcile(&reconciler, &desired);

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
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
        let desired = desired_state(vec![video.clone()]);

        let result = reconcile(&reconciler, &desired);

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
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
            video_file_repository.clone(),
            VideoReconciler::new(
                video_repository.clone(),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                task_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(MetadataGenerator::new(
                    Arc::new(FakeYoutubeMetadataRepository::default()),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(VideoRelocator::new(
                    Arc::new(SqliteVideoRepository::new(db.database())),
                    Arc::new(FakeVideoFileRepository::default()),
                    Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(VideoLock::default()),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
                video_category_assigner(db.database()),
            ),
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

    /// The `(output_dir, entry)` pair the fake records for one delete call.
    fn deleted(entry: &str) -> (PathBuf, String) {
        (PathBuf::from("/videos/my-playlist"), entry.to_string())
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn reconcile(
        reconciler: &InternalVideoReconciler,
        desired: &DesiredState,
    ) -> Result<(), String> {
        reconciler.reconcile(desired).map_err(|e| e.to_string())
    }

    fn video_category_assigner(database: Database) -> VideoCategoryAssigner {
        VideoCategoryAssigner::new(
            Arc::new(SqliteVideoCategoriesRepository::new(database.clone())),
            Arc::new(SqliteCategoryRepository::new(database.clone())),
            Arc::new(SqliteChannelVideoRepository::new(database.clone())),
            Arc::new(SqlitePlaylistVideoRepository::new(database.clone())),
            Arc::new(SqliteEventPublisher::new(
                database,
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(CategoryLock::default()),
        )
    }
}
