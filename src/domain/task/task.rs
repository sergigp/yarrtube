use super::errors::TaskError;
use super::task_lane::TaskLane;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Task {
    ReconcilePlaylist {
        playlist_id: String,
    },
    ReconcileChannel {
        channel_id: String,
    },
    DownloadVideo {
        video_id: String,
        quality: String,
        output_dir: String,
    },
    DeleteVideoFile {
        filename: Option<String>,
        thumbnail_filename: Option<String>,
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
        output_dir: String,
    },
    FetchSubtitles {
        video_id: String,
        output_dir: String,
    },
}

#[derive(Debug, Deserialize)]
struct ReconcilePlaylistPayload {
    playlist_id: String,
}

#[derive(Debug, Deserialize)]
struct ReconcileChannelPayload {
    channel_id: String,
}

#[derive(Debug, Deserialize)]
struct DownloadVideoPayload {
    video_id: String,
    quality: String,
    output_dir: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteVideoFilePayload {
    pub filename: Option<String>,
    pub thumbnail_filename: Option<String>,
    /// Absent from tasks queued before subtitles existed.
    #[serde(default)]
    pub subtitle_filename: Option<String>,
    pub output_dir: String,
}

#[derive(Debug, Deserialize)]
struct DeletePlaylistFilesPayload {
    playlist_id: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct DeleteChannelFilesPayload {
    channel_id: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct UpdateYtdlpPayload {}

#[derive(Debug, Deserialize)]
struct FetchThumbnailPayload {
    video_id: String,
    output_dir: String,
}

#[derive(Debug, Deserialize)]
struct FetchSubtitlesPayload {
    video_id: String,
    output_dir: String,
}

impl Task {
    pub fn task_type(&self) -> &'static str {
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
        }
    }

    pub fn payload(&self) -> Value {
        match self {
            Self::ReconcilePlaylist { playlist_id } => json!({ "playlist_id": playlist_id }),
            Self::ReconcileChannel { channel_id } => json!({ "channel_id": channel_id }),
            Self::DownloadVideo {
                video_id,
                quality,
                output_dir,
            } => json!({ "video_id": video_id, "quality": quality, "output_dir": output_dir }),
            Self::DeleteVideoFile {
                filename,
                thumbnail_filename,
                subtitle_filename,
                output_dir,
            } => json!({
                "filename": filename,
                "thumbnail_filename": thumbnail_filename,
                "subtitle_filename": subtitle_filename,
                "output_dir": output_dir,
            }),
            Self::DeletePlaylistFiles { playlist_id, path } => json!({
                "playlist_id": playlist_id,
                "path": path,
            }),
            Self::DeleteChannelFiles { channel_id, path } => json!({
                "channel_id": channel_id,
                "path": path,
            }),
            Self::UpdateYtdlp => json!({}),
            Self::FetchThumbnail {
                video_id,
                output_dir,
            } => json!({ "video_id": video_id, "output_dir": output_dir }),
            Self::FetchSubtitles {
                video_id,
                output_dir,
            } => json!({ "video_id": video_id, "output_dir": output_dir }),
        }
    }

    /// Decodes a `reconcile_playlist` task's raw JSON payload, as handed to a
    /// `TaskHandler`, back into the playlist ID it targets.
    pub fn decode_reconcile_playlist_payload(payload: &str) -> Result<String, TaskError> {
        let parsed: ReconcilePlaylistPayload = serde_json::from_str(payload)
            .map_err(|e| TaskError(format!("invalid reconcile_playlist payload: {e}")))?;
        Ok(parsed.playlist_id)
    }

    /// Decodes a `reconcile_channel` task's raw JSON payload, as handed to a
    /// `TaskHandler`, back into the channel handle it targets.
    pub fn decode_reconcile_channel_payload(payload: &str) -> Result<String, TaskError> {
        let parsed: ReconcileChannelPayload = serde_json::from_str(payload)
            .map_err(|e| TaskError(format!("invalid reconcile_channel payload: {e}")))?;
        Ok(parsed.channel_id)
    }

    /// Decodes a `download_video` task's raw JSON payload, as handed to a
    /// `TaskHandler`, back into the video's surrogate ID, quality, and
    /// output directory it targets.
    pub fn decode_download_video_payload(
        payload: &str,
    ) -> Result<(String, String, String), TaskError> {
        let parsed: DownloadVideoPayload = serde_json::from_str(payload)
            .map_err(|e| TaskError(format!("invalid download_video payload: {e}")))?;
        Ok((parsed.video_id, parsed.quality, parsed.output_dir))
    }

    /// Decodes a `delete_video_file` task's raw JSON payload, as handed to a
    /// `TaskHandler`, back into the recorded filename, recorded thumbnail
    /// filename, recorded subtitle filename, and output directory it
    /// targets.
    pub fn decode_delete_video_file_payload(
        payload: &str,
    ) -> Result<DeleteVideoFilePayload, TaskError> {
        serde_json::from_str(payload)
            .map_err(|e| TaskError(format!("invalid delete_video_file payload: {e}")))
    }

    /// Decodes a `delete_playlist_files` task's raw JSON payload, as handed
    /// to a `TaskHandler`, back into the playlist ID and output path it
    /// targets.
    pub fn decode_delete_playlist_files_payload(
        payload: &str,
    ) -> Result<(String, String), TaskError> {
        let parsed: DeletePlaylistFilesPayload = serde_json::from_str(payload)
            .map_err(|e| TaskError(format!("invalid delete_playlist_files payload: {e}")))?;
        Ok((parsed.playlist_id, parsed.path))
    }

    /// Decodes a `delete_channel_files` task's raw JSON payload, as handed
    /// to a `TaskHandler`, back into the channel handle and output path it
    /// targets.
    pub fn decode_delete_channel_files_payload(
        payload: &str,
    ) -> Result<(String, String), TaskError> {
        let parsed: DeleteChannelFilesPayload = serde_json::from_str(payload)
            .map_err(|e| TaskError(format!("invalid delete_channel_files payload: {e}")))?;
        Ok((parsed.channel_id, parsed.path))
    }

    /// Decodes an `update_ytdlp` task's raw JSON payload, as handed to a
    /// `TaskHandler`. The payload carries no fields; this only confirms it's
    /// well-formed.
    pub fn decode_update_ytdlp_payload(payload: &str) -> Result<(), TaskError> {
        serde_json::from_str::<UpdateYtdlpPayload>(payload)
            .map_err(|e| TaskError(format!("invalid update_ytdlp payload: {e}")))?;
        Ok(())
    }

    /// Decodes a `fetch_thumbnail` task's raw JSON payload, as handed to a
    /// `TaskHandler`, back into the video's surrogate ID and the output
    /// directory it targets.
    pub fn decode_fetch_thumbnail_payload(payload: &str) -> Result<(String, String), TaskError> {
        let parsed: FetchThumbnailPayload = serde_json::from_str(payload)
            .map_err(|e| TaskError(format!("invalid fetch_thumbnail payload: {e}")))?;
        Ok((parsed.video_id, parsed.output_dir))
    }

    /// Decodes a `fetch_subtitles` task's raw JSON payload, as handed to a
    /// `TaskHandler`, back into the video's surrogate ID and the output
    /// directory it targets.
    pub fn decode_fetch_subtitles_payload(payload: &str) -> Result<(String, String), TaskError> {
        let parsed: FetchSubtitlesPayload = serde_json::from_str(payload)
            .map_err(|e| TaskError(format!("invalid fetch_subtitles payload: {e}")))?;
        Ok((parsed.video_id, parsed.output_dir))
    }

    /// The executor lane a task of `task_type` runs in. Unknown types fall
    /// into `Light`.
    pub fn lane_for(task_type: &str) -> TaskLane {
        match task_type {
            "download_video" => TaskLane::Download,
            "fetch_thumbnail" => TaskLane::Thumbnail,
            "fetch_subtitles" => TaskLane::Subtitles,
            "update_ytdlp" => TaskLane::Exclusive,
            _ => TaskLane::Light,
        }
    }

    /// `Some("video:<id>")` for `download_video`, `fetch_thumbnail` and
    /// `fetch_subtitles`: two tasks sharing a key never run at the same
    /// time, and a task with a key is not scheduled twice while one is
    /// pending or running.
    pub fn exclusivity_key(task_type: &str, payload: &str) -> Option<String> {
        let video_id = match task_type {
            "download_video" => Self::decode_download_video_payload(payload)
                .ok()
                .map(|(video_id, _, _)| video_id),
            "fetch_thumbnail" => Self::decode_fetch_thumbnail_payload(payload)
                .ok()
                .map(|(video_id, _)| video_id),
            "fetch_subtitles" => Self::decode_fetch_subtitles_payload(payload)
                .ok()
                .map(|(video_id, _)| video_id),
            _ => None,
        }?;
        Some(format!("video:{video_id}"))
    }

    /// The show directory a per-video task (`download_video`,
    /// `fetch_thumbnail`, `fetch_subtitles`, `delete_video_file`) writes to
    /// or deletes from; `None` for any other task type.
    pub fn output_dir_of(task_type: &str, payload: &str) -> Option<String> {
        match task_type {
            "download_video" => Self::decode_download_video_payload(payload)
                .ok()
                .map(|(_, _, output_dir)| output_dir),
            "fetch_thumbnail" => Self::decode_fetch_thumbnail_payload(payload)
                .ok()
                .map(|(_, output_dir)| output_dir),
            "fetch_subtitles" => Self::decode_fetch_subtitles_payload(payload)
                .ok()
                .map(|(_, output_dir)| output_dir),
            "delete_video_file" => Self::decode_delete_video_file_payload(payload)
                .ok()
                .map(|payload| payload.output_dir),
            _ => None,
        }
    }

    /// The source path (relative to the videos root) a `delete_playlist_files`
    /// or `delete_channel_files` task removes recursively; `None` for any
    /// other task type.
    pub fn deleted_path_of(task_type: &str, payload: &str) -> Option<String> {
        match task_type {
            "delete_playlist_files" => Self::decode_delete_playlist_files_payload(payload)
                .ok()
                .map(|(_, path)| path),
            "delete_channel_files" => Self::decode_delete_channel_files_payload(payload)
                .ok()
                .map(|(_, path)| path),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_map_reconcile_playlist_to_a_stable_type_and_payload() {
        let task = Task::ReconcilePlaylist {
            playlist_id: "PL1".to_string(),
        };

        assert_eq!(task.task_type(), "reconcile_playlist");
        assert_eq!(task.payload(), json!({ "playlist_id": "PL1" }));
    }

    #[test]
    fn it_should_decode_a_reconcile_playlist_payload_back_into_its_playlist_id() {
        let task = Task::ReconcilePlaylist {
            playlist_id: "PL1".to_string(),
        };

        let playlist_id =
            Task::decode_reconcile_playlist_payload(&task.payload().to_string()).unwrap();

        assert_eq!(playlist_id, "PL1");
    }

    #[test]
    fn it_should_reject_a_malformed_reconcile_playlist_payload() {
        assert!(Task::decode_reconcile_playlist_payload("not json").is_err());
    }

    #[test]
    fn it_should_map_reconcile_channel_to_a_stable_type_and_payload() {
        let task = Task::ReconcileChannel {
            channel_id: "@somechannel".to_string(),
        };

        assert_eq!(task.task_type(), "reconcile_channel");
        assert_eq!(task.payload(), json!({ "channel_id": "@somechannel" }));
    }

    #[test]
    fn it_should_decode_a_reconcile_channel_payload_back_into_its_channel_id() {
        let task = Task::ReconcileChannel {
            channel_id: "@somechannel".to_string(),
        };

        let channel_id =
            Task::decode_reconcile_channel_payload(&task.payload().to_string()).unwrap();

        assert_eq!(channel_id, "@somechannel");
    }

    #[test]
    fn it_should_reject_a_malformed_reconcile_channel_payload() {
        assert!(Task::decode_reconcile_channel_payload("not json").is_err());
    }

    #[test]
    fn it_should_map_download_video_to_a_stable_type_and_payload() {
        let task = Task::DownloadVideo {
            video_id: "rec1".to_string(),
            quality: "high".to_string(),
            output_dir: "/videos/music".to_string(),
        };

        assert_eq!(task.task_type(), "download_video");
        assert_eq!(
            task.payload(),
            json!({ "video_id": "rec1", "quality": "high", "output_dir": "/videos/music" })
        );
    }

    #[test]
    fn it_should_decode_a_download_video_payload_back_into_its_fields() {
        let task = Task::DownloadVideo {
            video_id: "rec1".to_string(),
            quality: "high".to_string(),
            output_dir: "/videos/music".to_string(),
        };

        let (video_id, quality, output_dir) =
            Task::decode_download_video_payload(&task.payload().to_string()).unwrap();

        assert_eq!(video_id, "rec1");
        assert_eq!(quality, "high");
        assert_eq!(output_dir, "/videos/music");
    }

    #[test]
    fn it_should_reject_a_malformed_download_video_payload() {
        assert!(Task::decode_download_video_payload("not json").is_err());
    }

    #[test]
    fn it_should_map_delete_video_file_to_a_stable_type_and_payload() {
        let task = Task::DeleteVideoFile {
            filename: Some("My Video.mp4".to_string()),
            thumbnail_filename: Some("My Video.jpg".to_string()),
            subtitle_filename: Some("My Video.en.srt".to_string()),
            output_dir: "/videos/music".to_string(),
        };

        assert_eq!(task.task_type(), "delete_video_file");
        assert_eq!(
            task.payload(),
            json!({
                "filename": "My Video.mp4",
                "thumbnail_filename": "My Video.jpg",
                "subtitle_filename": "My Video.en.srt",
                "output_dir": "/videos/music",
            })
        );
    }

    #[test]
    fn it_should_decode_a_delete_video_file_payload_back_into_its_filename_and_output_dir() {
        let task = Task::DeleteVideoFile {
            filename: Some("My Video.mp4".to_string()),
            thumbnail_filename: Some("My Video.jpg".to_string()),
            subtitle_filename: Some("My Video.en.srt".to_string()),
            output_dir: "/videos/music".to_string(),
        };

        let DeleteVideoFilePayload {
            filename,
            thumbnail_filename,
            subtitle_filename,
            output_dir,
        } = Task::decode_delete_video_file_payload(&task.payload().to_string()).unwrap();

        assert_eq!(filename, Some("My Video.mp4".to_string()));
        assert_eq!(thumbnail_filename, Some("My Video.jpg".to_string()));
        assert_eq!(subtitle_filename, Some("My Video.en.srt".to_string()));
        assert_eq!(output_dir, "/videos/music");
    }

    #[test]
    fn it_should_reject_a_malformed_delete_video_file_payload() {
        assert!(Task::decode_delete_video_file_payload("not json").is_err());
    }

    #[test]
    fn it_should_map_delete_playlist_files_to_a_stable_type_and_payload() {
        let task = Task::DeletePlaylistFiles {
            playlist_id: "PL1".to_string(),
            path: "music/chill".to_string(),
        };

        assert_eq!(task.task_type(), "delete_playlist_files");
        assert_eq!(
            task.payload(),
            json!({ "playlist_id": "PL1", "path": "music/chill" })
        );
    }

    #[test]
    fn it_should_decode_a_delete_playlist_files_payload_back_into_its_playlist_id_and_path() {
        let task = Task::DeletePlaylistFiles {
            playlist_id: "PL1".to_string(),
            path: "music/chill".to_string(),
        };

        let (playlist_id, path) =
            Task::decode_delete_playlist_files_payload(&task.payload().to_string()).unwrap();

        assert_eq!(playlist_id, "PL1");
        assert_eq!(path, "music/chill");
    }

    #[test]
    fn it_should_reject_a_malformed_delete_playlist_files_payload() {
        assert!(Task::decode_delete_playlist_files_payload("not json").is_err());
    }

    #[test]
    fn it_should_map_delete_channel_files_to_a_stable_type_and_payload() {
        let task = Task::DeleteChannelFiles {
            channel_id: "@somechannel".to_string(),
            path: "creators/somechannel".to_string(),
        };

        assert_eq!(task.task_type(), "delete_channel_files");
        assert_eq!(
            task.payload(),
            json!({ "channel_id": "@somechannel", "path": "creators/somechannel" })
        );
    }

    #[test]
    fn it_should_decode_a_delete_channel_files_payload_back_into_its_channel_id_and_path() {
        let task = Task::DeleteChannelFiles {
            channel_id: "@somechannel".to_string(),
            path: "creators/somechannel".to_string(),
        };

        let (channel_id, path) =
            Task::decode_delete_channel_files_payload(&task.payload().to_string()).unwrap();

        assert_eq!(channel_id, "@somechannel");
        assert_eq!(path, "creators/somechannel");
    }

    #[test]
    fn it_should_reject_a_malformed_delete_channel_files_payload() {
        assert!(Task::decode_delete_channel_files_payload("not json").is_err());
    }

    #[test]
    fn it_should_map_update_ytdlp_to_a_stable_type_and_payload() {
        let task = Task::UpdateYtdlp;

        assert_eq!(task.task_type(), "update_ytdlp");
        assert_eq!(task.payload(), json!({}));
    }

    #[test]
    fn it_should_round_trip_an_update_ytdlp_payload() {
        let task = Task::UpdateYtdlp;

        assert!(Task::decode_update_ytdlp_payload(&task.payload().to_string()).is_ok());
    }

    #[test]
    fn it_should_reject_a_malformed_update_ytdlp_payload() {
        assert!(Task::decode_update_ytdlp_payload("not json").is_err());
    }

    #[test]
    fn it_should_map_task_types_to_lanes() {
        let lanes: Vec<TaskLane> = [
            "download_video",
            "fetch_thumbnail",
            "fetch_subtitles",
            "update_ytdlp",
            "reconcile_playlist",
            "reconcile_channel",
            "delete_video_file",
            "delete_playlist_files",
            "delete_channel_files",
            "unknown",
        ]
        .into_iter()
        .map(Task::lane_for)
        .collect();

        assert_eq!(
            lanes,
            vec![
                TaskLane::Download,
                TaskLane::Thumbnail,
                TaskLane::Subtitles,
                TaskLane::Exclusive,
                TaskLane::Light,
                TaskLane::Light,
                TaskLane::Light,
                TaskLane::Light,
                TaskLane::Light,
                TaskLane::Light,
            ]
        );
    }

    #[test]
    fn it_should_key_video_tasks_by_video_id() {
        let download = Task::DownloadVideo {
            video_id: "rec1".to_string(),
            quality: "high".to_string(),
            output_dir: "/videos/music".to_string(),
        };
        let fetch_thumbnail = Task::FetchThumbnail {
            video_id: "rec2".to_string(),
            output_dir: "/videos/music".to_string(),
        };
        let reconcile = Task::ReconcilePlaylist {
            playlist_id: "PL1".to_string(),
        };

        let keys: Vec<Option<String>> = [download, fetch_thumbnail, reconcile]
            .iter()
            .map(|task| Task::exclusivity_key(task.task_type(), &task.payload().to_string()))
            .collect();

        assert_eq!(
            keys,
            vec![
                Some("video:rec1".to_string()),
                Some("video:rec2".to_string()),
                None,
            ]
        );
    }

    #[test]
    fn it_should_read_the_output_dir_of_every_per_video_task() {
        let tasks = [
            Task::DownloadVideo {
                video_id: "rec1".to_string(),
                quality: "high".to_string(),
                output_dir: "/videos/music".to_string(),
            },
            Task::FetchThumbnail {
                video_id: "rec1".to_string(),
                output_dir: "/videos/music".to_string(),
            },
            Task::FetchSubtitles {
                video_id: "rec1".to_string(),
                output_dir: "/videos/music".to_string(),
            },
            Task::DeleteVideoFile {
                filename: Some("My Video.mp4".to_string()),
                thumbnail_filename: None,
                subtitle_filename: None,
                output_dir: "/videos/music".to_string(),
            },
        ];

        let output_dirs: Vec<Option<String>> = tasks
            .iter()
            .map(|task| Task::output_dir_of(task.task_type(), &task.payload().to_string()))
            .collect();

        assert_eq!(output_dirs, vec![Some("/videos/music".to_string()); 4]);
    }

    #[test]
    fn it_should_have_no_output_dir_for_reconciles() {
        let tasks = [
            Task::ReconcilePlaylist {
                playlist_id: "PL1".to_string(),
            },
            Task::ReconcileChannel {
                channel_id: "@somechannel".to_string(),
            },
        ];

        let output_dirs: Vec<Option<String>> = tasks
            .iter()
            .map(|task| Task::output_dir_of(task.task_type(), &task.payload().to_string()))
            .collect();

        assert_eq!(output_dirs, vec![None, None]);
    }

    #[test]
    fn it_should_read_the_deleted_path_of_source_file_deletions() {
        let tasks = [
            Task::DeletePlaylistFiles {
                playlist_id: "PL1".to_string(),
                path: "music/chill".to_string(),
            },
            Task::DeleteChannelFiles {
                channel_id: "@somechannel".to_string(),
                path: "music/chill".to_string(),
            },
        ];

        let deleted_paths: Vec<Option<String>> = tasks
            .iter()
            .map(|task| Task::deleted_path_of(task.task_type(), &task.payload().to_string()))
            .collect();

        assert_eq!(deleted_paths, vec![Some("music/chill".to_string()); 2]);
    }

    #[test]
    fn it_should_have_no_deleted_path_for_other_tasks() {
        let tasks = [
            Task::ReconcilePlaylist {
                playlist_id: "PL1".to_string(),
            },
            Task::DeleteVideoFile {
                filename: Some("My Video.mp4".to_string()),
                thumbnail_filename: None,
                subtitle_filename: None,
                output_dir: "/videos/music".to_string(),
            },
        ];

        let deleted_paths: Vec<Option<String>> = tasks
            .iter()
            .map(|task| Task::deleted_path_of(task.task_type(), &task.payload().to_string()))
            .collect();

        assert_eq!(deleted_paths, vec![None, None]);
    }

    #[test]
    fn it_should_have_no_deleted_path_for_an_undecodable_payload() {
        assert_eq!(Task::deleted_path_of("delete_playlist_files", "{}"), None);
    }
}
