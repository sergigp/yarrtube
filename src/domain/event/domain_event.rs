use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    PlaylistCreated {
        playlist_id: String,
    },
    PlaylistDeleted {
        playlist_id: String,
        path: String,
    },
    VideoAddedToPlaylist {
        playlist_id: String,
        video_id: String,
    },
    VideoRemovedFromPlaylist {
        playlist_id: String,
        video_id: String,
        title: String,
        filename: Option<String>,
        was_downloaded: bool,
    },
    ChannelCreated {
        channel_id: String,
    },
    ChannelDeleted {
        channel_id: String,
        path: String,
    },
    VideoAddedToChannel {
        channel_id: String,
        video_id: String,
    },
    VideoRemovedFromChannel {
        channel_id: String,
        video_id: String,
        title: String,
        filename: Option<String>,
        was_downloaded: bool,
    },
}

impl DomainEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::PlaylistCreated { .. } => "playlist_created",
            Self::PlaylistDeleted { .. } => "playlist_deleted",
            Self::VideoAddedToPlaylist { .. } => "video_added_to_playlist",
            Self::VideoRemovedFromPlaylist { .. } => "video_removed_from_playlist",
            Self::ChannelCreated { .. } => "channel_created",
            Self::ChannelDeleted { .. } => "channel_deleted",
            Self::VideoAddedToChannel { .. } => "video_added_to_channel",
            Self::VideoRemovedFromChannel { .. } => "video_removed_from_channel",
        }
    }

    pub fn payload(&self) -> Value {
        match self {
            Self::PlaylistCreated { playlist_id } => json!({ "playlist_id": playlist_id }),
            Self::PlaylistDeleted { playlist_id, path } => json!({
                "playlist_id": playlist_id,
                "path": path,
            }),
            Self::VideoAddedToPlaylist {
                playlist_id,
                video_id,
            } => json!({
                "playlist_id": playlist_id,
                "video_id": video_id,
            }),
            Self::VideoRemovedFromPlaylist {
                playlist_id,
                video_id,
                title,
                filename,
                was_downloaded,
            } => json!({
                "playlist_id": playlist_id,
                "video_id": video_id,
                "title": title,
                "filename": filename,
                "was_downloaded": was_downloaded,
            }),
            Self::ChannelCreated { channel_id } => json!({ "channel_id": channel_id }),
            Self::ChannelDeleted { channel_id, path } => json!({
                "channel_id": channel_id,
                "path": path,
            }),
            Self::VideoAddedToChannel {
                channel_id,
                video_id,
            } => json!({
                "channel_id": channel_id,
                "video_id": video_id,
            }),
            Self::VideoRemovedFromChannel {
                channel_id,
                video_id,
                title,
                filename,
                was_downloaded,
            } => json!({
                "channel_id": channel_id,
                "video_id": video_id,
                "title": title,
                "filename": filename,
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
            path: "music/chill".to_string(),
        };

        assert_eq!(event.event_type(), "playlist_deleted");
        assert_eq!(
            event.payload(),
            json!({ "playlist_id": "PL1", "path": "music/chill" })
        );
    }

    #[test]
    fn it_should_map_video_added_to_playlist_to_a_stable_type_and_payload() {
        let event = DomainEvent::VideoAddedToPlaylist {
            playlist_id: "PL1".to_string(),
            video_id: "rec1".to_string(),
        };

        assert_eq!(event.event_type(), "video_added_to_playlist");
        assert_eq!(
            event.payload(),
            json!({ "playlist_id": "PL1", "video_id": "rec1" })
        );
    }

    #[test]
    fn it_should_map_video_removed_from_playlist_to_a_stable_type_and_payload() {
        let event = DomainEvent::VideoRemovedFromPlaylist {
            playlist_id: "PL1".to_string(),
            video_id: "rec1".to_string(),
            title: "My Video".to_string(),
            filename: Some("My Video.mp4".to_string()),
            was_downloaded: true,
        };

        assert_eq!(event.event_type(), "video_removed_from_playlist");
        assert_eq!(
            event.payload(),
            json!({
                "playlist_id": "PL1",
                "video_id": "rec1",
                "title": "My Video",
                "filename": "My Video.mp4",
                "was_downloaded": true,
            })
        );
    }

    #[test]
    fn it_should_map_video_removed_from_playlist_with_no_recorded_filename() {
        let event = DomainEvent::VideoRemovedFromPlaylist {
            playlist_id: "PL1".to_string(),
            video_id: "rec1".to_string(),
            title: "My Video".to_string(),
            filename: None,
            was_downloaded: false,
        };

        assert_eq!(
            event.payload(),
            json!({
                "playlist_id": "PL1",
                "video_id": "rec1",
                "title": "My Video",
                "filename": null,
                "was_downloaded": false,
            })
        );
    }

    #[test]
    fn it_should_map_channel_created_to_a_stable_type_and_payload() {
        let event = DomainEvent::ChannelCreated {
            channel_id: "@somechannel".to_string(),
        };

        assert_eq!(event.event_type(), "channel_created");
        assert_eq!(event.payload(), json!({ "channel_id": "@somechannel" }));
    }

    #[test]
    fn it_should_map_channel_deleted_to_a_stable_type_and_payload() {
        let event = DomainEvent::ChannelDeleted {
            channel_id: "@somechannel".to_string(),
            path: "creators/somechannel".to_string(),
        };

        assert_eq!(event.event_type(), "channel_deleted");
        assert_eq!(
            event.payload(),
            json!({ "channel_id": "@somechannel", "path": "creators/somechannel" })
        );
    }

    #[test]
    fn it_should_map_video_added_to_channel_to_a_stable_type_and_payload() {
        let event = DomainEvent::VideoAddedToChannel {
            channel_id: "@somechannel".to_string(),
            video_id: "rec1".to_string(),
        };

        assert_eq!(event.event_type(), "video_added_to_channel");
        assert_eq!(
            event.payload(),
            json!({ "channel_id": "@somechannel", "video_id": "rec1" })
        );
    }

    #[test]
    fn it_should_map_video_removed_from_channel_to_a_stable_type_and_payload() {
        let event = DomainEvent::VideoRemovedFromChannel {
            channel_id: "@somechannel".to_string(),
            video_id: "rec1".to_string(),
            title: "My Video".to_string(),
            filename: Some("My Video.mp4".to_string()),
            was_downloaded: true,
        };

        assert_eq!(event.event_type(), "video_removed_from_channel");
        assert_eq!(
            event.payload(),
            json!({
                "channel_id": "@somechannel",
                "video_id": "rec1",
                "title": "My Video",
                "filename": "My Video.mp4",
                "was_downloaded": true,
            })
        );
    }
}
