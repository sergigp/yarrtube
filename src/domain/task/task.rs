use super::errors::TaskError;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Task {
    SyncPlaylist { playlist_id: String },
}

#[derive(Debug, Deserialize)]
struct SyncPlaylistPayload {
    playlist_id: String,
}

impl Task {
    pub fn task_type(&self) -> &'static str {
        match self {
            Self::SyncPlaylist { .. } => "sync_playlist",
        }
    }

    pub fn payload(&self) -> Value {
        match self {
            Self::SyncPlaylist { playlist_id } => json!({ "playlist_id": playlist_id }),
        }
    }

    /// Decodes a `sync_playlist` task's raw JSON payload, as handed to a
    /// `TaskHandler`, back into the playlist ID it targets.
    pub fn decode_sync_playlist_payload(payload: &str) -> Result<String, TaskError> {
        let parsed: SyncPlaylistPayload = serde_json::from_str(payload)
            .map_err(|e| TaskError(format!("invalid sync_playlist payload: {e}")))?;
        Ok(parsed.playlist_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_map_sync_playlist_to_a_stable_type_and_payload() {
        let task = Task::SyncPlaylist {
            playlist_id: "PL1".to_string(),
        };

        assert_eq!(task.task_type(), "sync_playlist");
        assert_eq!(task.payload(), json!({ "playlist_id": "PL1" }));
    }

    #[test]
    fn it_should_decode_a_sync_playlist_payload_back_into_its_playlist_id() {
        let task = Task::SyncPlaylist {
            playlist_id: "PL1".to_string(),
        };

        let playlist_id = Task::decode_sync_playlist_payload(&task.payload().to_string()).unwrap();

        assert_eq!(playlist_id, "PL1");
    }

    #[test]
    fn it_should_reject_a_malformed_sync_playlist_payload() {
        assert!(Task::decode_sync_playlist_payload("not json").is_err());
    }
}
