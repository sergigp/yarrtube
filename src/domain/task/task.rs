use super::task_lane::TaskLane;
use serde::{Deserialize, Serialize};

/// A unit of background work. Stored as its `task_type` (the serde tag, the
/// variant's snake_case name) plus the remaining fields as a JSON payload;
/// `SqliteTaskRepository` encodes and decodes it, so that format is the one
/// existing databases hold and must not change.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "task_type", rename_all = "snake_case")]
pub enum Task {
    ReconcilePlaylist {
        playlist_id: String,
    },
    ReconcileChannel {
        channel_id: String,
    },
    /// Quality and show folder are resolved through the video's owner when
    /// the task runs. Rows queued before that still carry `quality` and
    /// `output_dir`; they decode, the extra keys ignored.
    DownloadVideo {
        video_id: String,
    },
    DeleteVideoFile {
        filename: Option<String>,
        thumbnail_filename: Option<String>,
        /// Absent from tasks queued before subtitles existed.
        #[serde(default)]
        subtitle_filename: Option<String>,
        output_dir: String,
    },
    DeletePlaylistFiles {
        playlist_id: String,
        path: String,
    },
    DeleteChannelFiles {
        channel_id: String,
        path: String,
    },
    UpdateYtdlp,
    FetchThumbnail {
        video_id: String,
    },
    FetchSubtitles {
        video_id: String,
    },
    SyncVideo {
        video_id: String,
    },
    SyncPlexLabels,
    SyncPlexWatchState,
    FetchChapters {
        video_id: String,
    },
    UpgradeQuality {
        video_id: String,
    },
    SummarizeVideo {
        video_id: String,
    },
    CategorizeVideo {
        video_id: String,
    },
    /// A stored task this version can't decode: an unknown `task_type`, or a
    /// payload that doesn't match its type. Kept verbatim so it is stored
    /// back unchanged; the executor fails every attempt at it, so it is
    /// retried and then dead-lettered like any other failing task, never
    /// dropped. Never scheduled by the application itself.
    #[serde(skip)]
    Unreadable {
        task_type: String,
        payload: String,
        error: String,
    },
}

impl Task {
    pub fn task_type(&self) -> &str {
        match self {
            Self::ReconcilePlaylist { .. } => "reconcile_playlist",
            Self::ReconcileChannel { .. } => "reconcile_channel",
            Self::DownloadVideo { .. } => "download_video",
            Self::DeleteVideoFile { .. } => "delete_video_file",
            Self::DeletePlaylistFiles { .. } => "delete_playlist_files",
            Self::DeleteChannelFiles { .. } => "delete_channel_files",
            Self::UpdateYtdlp => "update_ytdlp",
            Self::FetchThumbnail { .. } => "fetch_thumbnail",
            Self::FetchSubtitles { .. } => "fetch_subtitles",
            Self::SyncVideo { .. } => "sync_video",
            Self::SyncPlexLabels => "sync_plex_labels",
            Self::SyncPlexWatchState => "sync_plex_watch_state",
            Self::FetchChapters { .. } => "fetch_chapters",
            Self::SummarizeVideo { .. } => "summarize_video",
            Self::CategorizeVideo { .. } => "categorize_video",
            Self::UpgradeQuality { .. } => "upgrade_quality",
            Self::Unreadable { task_type, .. } => task_type,
        }
    }

    /// The executor lane this task runs in. An unreadable task runs in
    /// `Light`.
    pub fn lane(&self) -> TaskLane {
        match self {
            Self::DownloadVideo { .. } | Self::UpgradeQuality { .. } => TaskLane::Download,
            Self::FetchThumbnail { .. } => TaskLane::Thumbnail,
            Self::FetchSubtitles { .. } | Self::FetchChapters { .. } => TaskLane::Subtitles,
            Self::SyncVideo { .. } => TaskLane::VideoSync,
            Self::SyncPlexLabels | Self::SyncPlexWatchState => TaskLane::Plex,
            Self::SummarizeVideo { .. } | Self::CategorizeVideo { .. } => TaskLane::Summaries,
            Self::UpdateYtdlp => TaskLane::Exclusive,
            Self::ReconcilePlaylist { .. }
            | Self::ReconcileChannel { .. }
            | Self::DeleteVideoFile { .. }
            | Self::DeletePlaylistFiles { .. }
            | Self::DeleteChannelFiles { .. }
            | Self::Unreadable { .. } => TaskLane::Light,
        }
    }

    /// `Some("video:<id>")` for every task about one video except
    /// `delete_video_file`: two tasks sharing a key never run at the same
    /// time, and a task with a key is not scheduled twice while one is
    /// pending (or running, see `dedupes_running`).
    pub fn exclusivity_key(&self) -> Option<String> {
        self.video_id().map(|video_id| format!("video:{video_id}"))
    }

    /// The video this task is about, if any; every task about one video
    /// except `delete_video_file`, which only carries its files.
    pub fn video_id(&self) -> Option<&str> {
        match self {
            Self::DownloadVideo { video_id }
            | Self::FetchThumbnail { video_id }
            | Self::FetchSubtitles { video_id }
            | Self::SyncVideo { video_id }
            | Self::FetchChapters { video_id }
            | Self::SummarizeVideo { video_id }
            | Self::CategorizeVideo { video_id }
            | Self::UpgradeQuality { video_id } => Some(video_id),
            _ => None,
        }
    }

    /// The channel this task is about, if any.
    pub fn channel_id(&self) -> Option<&str> {
        match self {
            Self::ReconcileChannel { channel_id } | Self::DeleteChannelFiles { channel_id, .. } => {
                Some(channel_id)
            }
            _ => None,
        }
    }

    /// The playlist this task is about, if any.
    pub fn playlist_id(&self) -> Option<&str> {
        match self {
            Self::ReconcilePlaylist { playlist_id }
            | Self::DeletePlaylistFiles { playlist_id, .. } => Some(playlist_id),
            _ => None,
        }
    }

    /// Whether scheduling a keyed task is skipped while one of the same type
    /// for the same video is running, not only pending. False for
    /// `sync_video`: a running sync must not swallow a newer request.
    pub fn dedupes_running(&self) -> bool {
        !matches!(self, Self::SyncVideo { .. })
    }

    /// The show directory a `delete_video_file` task deletes from; `None`
    /// for any other. The other per-video tasks write into their video's
    /// owner's show directory, resolved when they run (see
    /// `show_writing_video_id`).
    pub fn output_dir(&self) -> Option<&str> {
        match self {
            Self::DeleteVideoFile { output_dir, .. } => Some(output_dir),
            _ => None,
        }
    }

    /// The video whose show directory this task writes into (or, for
    /// `sync_video`, may have read before a move): `download_video`,
    /// `fetch_thumbnail`, `fetch_subtitles`, `upgrade_quality` and
    /// `sync_video`; `None` for any other.
    pub fn show_writing_video_id(&self) -> Option<&str> {
        match self {
            Self::DownloadVideo { video_id }
            | Self::FetchThumbnail { video_id }
            | Self::FetchSubtitles { video_id }
            | Self::UpgradeQuality { video_id }
            | Self::SyncVideo { video_id } => Some(video_id),
            _ => None,
        }
    }

    /// The source path (relative to the videos root) a `delete_playlist_files`
    /// or `delete_channel_files` task removes recursively; `None` for any
    /// other.
    pub fn deleted_path(&self) -> Option<&str> {
        match self {
            Self::DeletePlaylistFiles { path, .. } | Self::DeleteChannelFiles { path, .. } => {
                Some(path)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_map_tasks_to_lanes() {
        let lanes: Vec<TaskLane> = every_task().iter().map(Task::lane).collect();

        assert_eq!(
            lanes,
            vec![
                TaskLane::Light,
                TaskLane::Light,
                TaskLane::Download,
                TaskLane::Light,
                TaskLane::Light,
                TaskLane::Light,
                TaskLane::Exclusive,
                TaskLane::Thumbnail,
                TaskLane::Subtitles,
                TaskLane::VideoSync,
                TaskLane::Plex,
                TaskLane::Plex,
                TaskLane::Subtitles,
                TaskLane::Download,
                TaskLane::Summaries,
                TaskLane::Summaries,
                TaskLane::Light,
            ]
        );
    }

    #[test]
    fn it_should_key_video_tasks_by_video_id() {
        let keys: Vec<Option<String>> = every_task().iter().map(Task::exclusivity_key).collect();

        let video_key = Some("video:rec1".to_string());
        assert_eq!(
            keys,
            vec![
                None,
                None,
                video_key.clone(),
                None,
                None,
                None,
                None,
                video_key.clone(),
                video_key.clone(),
                video_key.clone(),
                None,
                None,
                video_key.clone(),
                video_key.clone(),
                video_key.clone(),
                video_key,
                None,
            ]
        );
    }

    #[test]
    fn it_should_dedupe_running_video_tasks_except_video_syncs() {
        let tasks = [
            Task::DownloadVideo {
                video_id: "rec1".to_string(),
            },
            Task::FetchThumbnail {
                video_id: "rec1".to_string(),
            },
            Task::FetchSubtitles {
                video_id: "rec1".to_string(),
            },
            Task::SyncVideo {
                video_id: "rec1".to_string(),
            },
        ];

        let dedupes: Vec<bool> = tasks.iter().map(Task::dedupes_running).collect();

        assert_eq!(dedupes, vec![true, true, true, false]);
    }

    #[test]
    fn it_should_read_the_output_dir_of_a_video_file_deletion_only() {
        let tasks = every_task();

        let output_dirs: Vec<Option<&str>> = tasks.iter().map(Task::output_dir).collect();

        let music = Some("/videos/music");
        assert_eq!(
            output_dirs,
            vec![
                None, None, None, music, None, None, None, None, None, None, None, None, None,
                None, None, None, None,
            ]
        );
    }

    #[test]
    fn it_should_read_the_video_of_every_task_writing_into_its_show_folder() {
        let tasks = every_task();

        let video_ids: Vec<Option<&str>> = tasks.iter().map(Task::show_writing_video_id).collect();

        let rec1 = Some("rec1");
        assert_eq!(
            video_ids,
            vec![
                None, None, rec1, None, None, None, None, rec1, rec1, rec1, None, None, None, rec1,
                None, None, None,
            ]
        );
    }

    #[test]
    fn it_should_read_the_deleted_path_of_source_file_deletions() {
        let tasks = every_task();

        let deleted_paths: Vec<Option<&str>> = tasks.iter().map(Task::deleted_path).collect();

        let chill = Some("music/chill");
        assert_eq!(
            deleted_paths,
            vec![
                None, None, None, None, chill, chill, None, None, None, None, None, None, None,
                None, None, None, None,
            ]
        );
    }

    #[test]
    fn it_should_name_an_unreadable_task_by_its_stored_type() {
        let task = Task::Unreadable {
            task_type: "reconcile_plex_collections".to_string(),
            payload: "{}".to_string(),
            error: "unknown variant".to_string(),
        };

        assert_eq!(task.task_type(), "reconcile_plex_collections");
    }

    /// One task of every variant, in declaration order.
    fn every_task() -> Vec<Task> {
        vec![
            Task::ReconcilePlaylist {
                playlist_id: "PL1".to_string(),
            },
            Task::ReconcileChannel {
                channel_id: "@somechannel".to_string(),
            },
            Task::DownloadVideo {
                video_id: "rec1".to_string(),
            },
            Task::DeleteVideoFile {
                filename: Some("My Video.mp4".to_string()),
                thumbnail_filename: None,
                subtitle_filename: None,
                output_dir: "/videos/music".to_string(),
            },
            Task::DeletePlaylistFiles {
                playlist_id: "PL1".to_string(),
                path: "music/chill".to_string(),
            },
            Task::DeleteChannelFiles {
                channel_id: "@somechannel".to_string(),
                path: "music/chill".to_string(),
            },
            Task::UpdateYtdlp,
            Task::FetchThumbnail {
                video_id: "rec1".to_string(),
            },
            Task::FetchSubtitles {
                video_id: "rec1".to_string(),
            },
            Task::SyncVideo {
                video_id: "rec1".to_string(),
            },
            Task::SyncPlexLabels,
            Task::SyncPlexWatchState,
            Task::FetchChapters {
                video_id: "rec1".to_string(),
            },
            Task::UpgradeQuality {
                video_id: "rec1".to_string(),
            },
            Task::SummarizeVideo {
                video_id: "rec1".to_string(),
            },
            Task::CategorizeVideo {
                video_id: "rec1".to_string(),
            },
            Task::Unreadable {
                task_type: "download_video".to_string(),
                payload: "{}".to_string(),
                error: "missing field `video_id`".to_string(),
            },
        ]
    }
}
