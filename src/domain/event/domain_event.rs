use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    PlaylistCreated {
        playlist_id: String,
    },
    PlaylistDeleted {
        playlist_id: String,
    },
    VideoAdded {
        playlist_id: String,
        video_id: String,
    },
    VideoDeleted {
        playlist_id: String,
        video_id: String,
        title: String,
        was_downloaded: bool,
    },
}

impl DomainEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::PlaylistCreated { .. } => "playlist_created",
            Self::PlaylistDeleted { .. } => "playlist_deleted",
            Self::VideoAdded { .. } => "video_added",
            Self::VideoDeleted { .. } => "video_deleted",
        }
    }

    pub fn payload(&self) -> Value {
        match self {
            Self::PlaylistCreated { playlist_id } => json!({ "playlist_id": playlist_id }),
            Self::PlaylistDeleted { playlist_id } => json!({ "playlist_id": playlist_id }),
            Self::VideoAdded {
                playlist_id,
                video_id,
            } => json!({
                "playlist_id": playlist_id,
                "video_id": video_id,
            }),
            Self::VideoDeleted {
                playlist_id,
                video_id,
                title,
                was_downloaded,
            } => json!({
                "playlist_id": playlist_id,
                "video_id": video_id,
                "title": title,
                "was_downloaded": was_downloaded,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_map_playlist_created_to_a_stable_type_and_payload() {
        let event = DomainEvent::PlaylistCreated {
            playlist_id: "PL1".to_string(),
        };

        assert_eq!(event.event_type(), "playlist_created");
        assert_eq!(event.payload(), json!({ "playlist_id": "PL1" }));
    }

    #[test]
    fn it_should_map_playlist_deleted_to_a_stable_type_and_payload() {
        let event = DomainEvent::PlaylistDeleted {
            playlist_id: "PL1".to_string(),
        };

        assert_eq!(event.event_type(), "playlist_deleted");
        assert_eq!(event.payload(), json!({ "playlist_id": "PL1" }));
    }

    #[test]
    fn it_should_map_video_added_to_a_stable_type_and_payload() {
        let event = DomainEvent::VideoAdded {
            playlist_id: "PL1".to_string(),
            video_id: "vid1".to_string(),
        };

        assert_eq!(event.event_type(), "video_added");
        assert_eq!(
            event.payload(),
            json!({ "playlist_id": "PL1", "video_id": "vid1" })
        );
    }

    #[test]
    fn it_should_map_video_deleted_to_a_stable_type_and_payload() {
        let event = DomainEvent::VideoDeleted {
            playlist_id: "PL1".to_string(),
            video_id: "vid1".to_string(),
            title: "My Video".to_string(),
            was_downloaded: true,
        };

        assert_eq!(event.event_type(), "video_deleted");
        assert_eq!(
            event.payload(),
            json!({
                "playlist_id": "PL1",
                "video_id": "vid1",
                "title": "My Video",
                "was_downloaded": true,
            })
        );
    }
}
