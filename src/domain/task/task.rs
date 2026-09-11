use super::errors::TaskError;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Task {
    ReconcilePlaylist {
        playlist_id: String,
    },
    DownloadVideo {
        playlist_id: String,
        video_id: String,
        quality: String,
    },
    DeleteVideoFile {
        playlist_id: String,
        video_id: String,
        title: String,
        filename: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct ReconcilePlaylistPayload {
    playlist_id: String,
}

#[derive(Debug, Deserialize)]
struct DownloadVideoPayload {
    playlist_id: String,
    video_id: String,
    quality: String,
}

#[derive(Debug, Deserialize)]
struct DeleteVideoFilePayload {
    playlist_id: String,
    video_id: String,
    title: String,
    filename: Option<String>,
}

impl Task {
    pub fn task_type(&self) -> &'static str {
        match self {
            Self::ReconcilePlaylist { .. } => "reconcile_playlist",
            Self::DownloadVideo { .. } => "download_video",
            Self::DeleteVideoFile { .. } => "delete_video_file",
        }
    }

    pub fn payload(&self) -> Value {
        match self {
            Self::ReconcilePlaylist { playlist_id } => json!({ "playlist_id": playlist_id }),
            Self::DownloadVideo {
                playlist_id,
                video_id,
                quality,
            } => json!({ "playlist_id": playlist_id, "video_id": video_id, "quality": quality }),
            Self::DeleteVideoFile {
                playlist_id,
                video_id,
                title,
                filename,
            } => json!({
                "playlist_id": playlist_id,
                "video_id": video_id,
                "title": title,
                "filename": filename,
            }),
        }
    }

    /// Decodes a `reconcile_playlist` task's raw JSON payload, as handed to a
    /// `TaskHandler`, back into the playlist ID it targets.
    pub fn decode_reconcile_playlist_payload(payload: &str) -> Result<String, TaskError> {
        let parsed: ReconcilePlaylistPayload = serde_json::from_str(payload)
            .map_err(|e| TaskError(format!("invalid reconcile_playlist payload: {e}")))?;
        Ok(parsed.playlist_id)
    }

    /// Decodes a `download_video` task's raw JSON payload, as handed to a
    /// `TaskHandler`, back into the playlist ID, video ID, and quality it targets.
    pub fn decode_download_video_payload(
        payload: &str,
    ) -> Result<(String, String, String), TaskError> {
        let parsed: DownloadVideoPayload = serde_json::from_str(payload)
            .map_err(|e| TaskError(format!("invalid download_video payload: {e}")))?;
        Ok((parsed.playlist_id, parsed.video_id, parsed.quality))
    }

    /// Decodes a `delete_video_file` task's raw JSON payload, as handed to a
    /// `TaskHandler`, back into the playlist ID, video ID, title, and
    /// recorded filename it targets.
    pub fn decode_delete_video_file_payload(
        payload: &str,
    ) -> Result<(String, String, String, Option<String>), TaskError> {
        let parsed: DeleteVideoFilePayload = serde_json::from_str(payload)
            .map_err(|e| TaskError(format!("invalid delete_video_file payload: {e}")))?;
        Ok((
            parsed.playlist_id,
            parsed.video_id,
            parsed.title,
            parsed.filename,
        ))
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
    fn it_should_map_download_video_to_a_stable_type_and_payload() {
        let task = Task::DownloadVideo {
            playlist_id: "PL1".to_string(),
            video_id: "vid1".to_string(),
            quality: "high".to_string(),
        };

        assert_eq!(task.task_type(), "download_video");
        assert_eq!(
            task.payload(),
            json!({ "playlist_id": "PL1", "video_id": "vid1", "quality": "high" })
        );
    }

    #[test]
    fn it_should_decode_a_download_video_payload_back_into_its_ids_and_quality() {
        let task = Task::DownloadVideo {
            playlist_id: "PL1".to_string(),
            video_id: "vid1".to_string(),
            quality: "high".to_string(),
        };

        let (playlist_id, video_id, quality) =
            Task::decode_download_video_payload(&task.payload().to_string()).unwrap();

        assert_eq!(playlist_id, "PL1");
        assert_eq!(video_id, "vid1");
        assert_eq!(quality, "high");
    }

    #[test]
    fn it_should_reject_a_malformed_download_video_payload() {
        assert!(Task::decode_download_video_payload("not json").is_err());
    }

    #[test]
    fn it_should_map_delete_video_file_to_a_stable_type_and_payload() {
        let task = Task::DeleteVideoFile {
            playlist_id: "PL1".to_string(),
            video_id: "vid1".to_string(),
            title: "My Video".to_string(),
            filename: Some("My Video.mp4".to_string()),
        };

        assert_eq!(task.task_type(), "delete_video_file");
        assert_eq!(
            task.payload(),
            json!({
                "playlist_id": "PL1",
                "video_id": "vid1",
                "title": "My Video",
                "filename": "My Video.mp4",
            })
        );
    }

    #[test]
    fn it_should_decode_a_delete_video_file_payload_back_into_its_ids_title_and_filename() {
        let task = Task::DeleteVideoFile {
            playlist_id: "PL1".to_string(),
            video_id: "vid1".to_string(),
            title: "My Video".to_string(),
            filename: Some("My Video.mp4".to_string()),
        };

        let (playlist_id, video_id, title, filename) =
            Task::decode_delete_video_file_payload(&task.payload().to_string()).unwrap();

        assert_eq!(playlist_id, "PL1");
        assert_eq!(video_id, "vid1");
        assert_eq!(title, "My Video");
        assert_eq!(filename, Some("My Video.mp4".to_string()));
    }

    #[test]
    fn it_should_reject_a_malformed_delete_video_file_payload() {
        assert!(Task::decode_delete_video_file_payload("not json").is_err());
    }
}
