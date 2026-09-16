use super::errors::TaskError;
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
struct DeleteVideoFilePayload {
    filename: Option<String>,
    thumbnail_filename: Option<String>,
    output_dir: String,
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
                output_dir,
            } => json!({
                "filename": filename,
                "thumbnail_filename": thumbnail_filename,
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
    /// filename, and output directory it targets.
    pub fn decode_delete_video_file_payload(
        payload: &str,
    ) -> Result<(Option<String>, Option<String>, String), TaskError> {
        let parsed: DeleteVideoFilePayload = serde_json::from_str(payload)
            .map_err(|e| TaskError(format!("invalid delete_video_file payload: {e}")))?;
        Ok((
            parsed.filename,
            parsed.thumbnail_filename,
            parsed.output_dir,
        ))
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
            output_dir: "/videos/music".to_string(),
        };

        assert_eq!(task.task_type(), "delete_video_file");
        assert_eq!(
            task.payload(),
            json!({
                "filename": "My Video.mp4",
                "thumbnail_filename": "My Video.jpg",
                "output_dir": "/videos/music",
            })
        );
    }

    #[test]
    fn it_should_decode_a_delete_video_file_payload_back_into_its_filename_and_output_dir() {
        let task = Task::DeleteVideoFile {
            filename: Some("My Video.mp4".to_string()),
            thumbnail_filename: Some("My Video.jpg".to_string()),
            output_dir: "/videos/music".to_string(),
        };

        let (filename, thumbnail_filename, output_dir) =
            Task::decode_delete_video_file_payload(&task.payload().to_string()).unwrap();

        assert_eq!(filename, Some("My Video.mp4".to_string()));
        assert_eq!(thumbnail_filename, Some("My Video.jpg".to_string()));
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
}
