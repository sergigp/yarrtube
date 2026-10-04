use crate::domain::channel::{ChannelCreated, ChannelDeleted};
use crate::domain::channel_video::{VideoAddedToChannel, VideoRemovedFromChannel};
use crate::domain::playlist::{PlaylistCreated, PlaylistDeleted};
use crate::domain::playlist_video::{VideoAddedToPlaylist, VideoRemovedFromPlaylist};
use crate::domain::video::VideoDownloaded;
use serde::Serialize;
use serde_json::Value;

/// Envelope over every aggregate's events, the unit the event publisher
/// persists and subscribers are routed by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    PlaylistCreated(PlaylistCreated),
    PlaylistDeleted(PlaylistDeleted),
    VideoAddedToPlaylist(VideoAddedToPlaylist),
    VideoRemovedFromPlaylist(VideoRemovedFromPlaylist),
    ChannelCreated(ChannelCreated),
    ChannelDeleted(ChannelDeleted),
    VideoAddedToChannel(VideoAddedToChannel),
    VideoRemovedFromChannel(VideoRemovedFromChannel),
    VideoDownloaded(VideoDownloaded),
}

impl DomainEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::PlaylistCreated(_) => PlaylistCreated::EVENT_TYPE,
            Self::PlaylistDeleted(_) => PlaylistDeleted::EVENT_TYPE,
            Self::VideoAddedToPlaylist(_) => VideoAddedToPlaylist::EVENT_TYPE,
            Self::VideoRemovedFromPlaylist(_) => VideoRemovedFromPlaylist::EVENT_TYPE,
            Self::ChannelCreated(_) => ChannelCreated::EVENT_TYPE,
            Self::ChannelDeleted(_) => ChannelDeleted::EVENT_TYPE,
            Self::VideoAddedToChannel(_) => VideoAddedToChannel::EVENT_TYPE,
            Self::VideoRemovedFromChannel(_) => VideoRemovedFromChannel::EVENT_TYPE,
            Self::VideoDownloaded(_) => VideoDownloaded::EVENT_TYPE,
        }
    }

    pub fn payload(&self) -> Value {
        match self {
            Self::PlaylistCreated(event) => to_payload(event),
            Self::PlaylistDeleted(event) => to_payload(event),
            Self::VideoAddedToPlaylist(event) => to_payload(event),
            Self::VideoRemovedFromPlaylist(event) => to_payload(event),
            Self::ChannelCreated(event) => to_payload(event),
            Self::ChannelDeleted(event) => to_payload(event),
            Self::VideoAddedToChannel(event) => to_payload(event),
            Self::VideoRemovedFromChannel(event) => to_payload(event),
            Self::VideoDownloaded(event) => to_payload(event),
        }
    }
}

fn to_payload(event: &impl Serialize) -> Value {
    serde_json::to_value(event).expect("domain events serialize to JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn it_should_map_playlist_created_to_a_stable_type_and_payload() {
        let event = DomainEvent::PlaylistCreated(PlaylistCreated {
            playlist_id: "PL1".to_string(),
        });

        assert_eq!(event.event_type(), "playlist_created");
        assert_eq!(event.payload(), json!({ "playlist_id": "PL1" }));
    }

    #[test]
    fn it_should_map_video_downloaded_to_a_stable_type_and_payload() {
        let event = DomainEvent::VideoDownloaded(VideoDownloaded {
            video_id: "rec1".to_string(),
            output_dir: "/videos/playlists/kids".to_string(),
            folder: "My Video".to_string(),
        });

        assert_eq!(event.event_type(), "video_downloaded");
        assert_eq!(
            event.payload(),
            json!({
                "video_id": "rec1",
                "output_dir": "/videos/playlists/kids",
                "folder": "My Video",
            })
        );
    }

    #[test]
    fn it_should_map_playlist_deleted_to_a_stable_type_and_payload() {
        let event = DomainEvent::PlaylistDeleted(PlaylistDeleted {
            playlist_id: "PL1".to_string(),
            name: "Lofi beats".to_string(),
            path: "music/chill".to_string(),
        });

        assert_eq!(event.event_type(), "playlist_deleted");
        assert_eq!(
            event.payload(),
            json!({ "playlist_id": "PL1", "name": "Lofi beats", "path": "music/chill" })
        );
    }

    #[test]
    fn it_should_map_video_added_to_playlist_to_a_stable_type_and_payload() {
        let event = DomainEvent::VideoAddedToPlaylist(VideoAddedToPlaylist {
            playlist_id: "PL1".to_string(),
            video_id: "rec1".to_string(),
        });

        assert_eq!(event.event_type(), "video_added_to_playlist");
        assert_eq!(
            event.payload(),
            json!({ "playlist_id": "PL1", "video_id": "rec1" })
        );
    }

    #[test]
    fn it_should_map_video_removed_from_playlist_to_a_stable_type_and_payload() {
        let event = DomainEvent::VideoRemovedFromPlaylist(VideoRemovedFromPlaylist {
            playlist_id: "PL1".to_string(),
            video_id: "rec1".to_string(),
            title: "My Video".to_string(),
            filename: Some("My Video.mp4".to_string()),
            thumbnail_filename: Some("My Video.jpg".to_string()),
            was_downloaded: true,
        });

        assert_eq!(event.event_type(), "video_removed_from_playlist");
        assert_eq!(
            event.payload(),
            json!({
                "playlist_id": "PL1",
                "video_id": "rec1",
                "title": "My Video",
                "filename": "My Video.mp4",
                "thumbnail_filename": "My Video.jpg",
                "was_downloaded": true,
            })
        );
    }

    #[test]
    fn it_should_map_video_removed_from_playlist_with_no_recorded_filename() {
        let event = DomainEvent::VideoRemovedFromPlaylist(VideoRemovedFromPlaylist {
            playlist_id: "PL1".to_string(),
            video_id: "rec1".to_string(),
            title: "My Video".to_string(),
            filename: None,
            thumbnail_filename: None,
            was_downloaded: false,
        });

        assert_eq!(
            event.payload(),
            json!({
                "playlist_id": "PL1",
                "video_id": "rec1",
                "title": "My Video",
                "filename": null,
                "thumbnail_filename": null,
                "was_downloaded": false,
            })
        );
    }

    #[test]
    fn it_should_map_channel_created_to_a_stable_type_and_payload() {
        let event = DomainEvent::ChannelCreated(ChannelCreated {
            channel_id: "@somechannel".to_string(),
        });

        assert_eq!(event.event_type(), "channel_created");
        assert_eq!(event.payload(), json!({ "channel_id": "@somechannel" }));
    }

    #[test]
    fn it_should_map_channel_deleted_to_a_stable_type_and_payload() {
        let event = DomainEvent::ChannelDeleted(ChannelDeleted {
            channel_id: "@somechannel".to_string(),
            name: "Some Channel".to_string(),
            path: "creators/somechannel".to_string(),
        });

        assert_eq!(event.event_type(), "channel_deleted");
        assert_eq!(
            event.payload(),
            json!({
                "channel_id": "@somechannel",
                "name": "Some Channel",
                "path": "creators/somechannel",
            })
        );
    }

    #[test]
    fn it_should_map_video_added_to_channel_to_a_stable_type_and_payload() {
        let event = DomainEvent::VideoAddedToChannel(VideoAddedToChannel {
            channel_id: "@somechannel".to_string(),
            video_id: "rec1".to_string(),
        });

        assert_eq!(event.event_type(), "video_added_to_channel");
        assert_eq!(
            event.payload(),
            json!({ "channel_id": "@somechannel", "video_id": "rec1" })
        );
    }

    #[test]
    fn it_should_map_video_removed_from_channel_to_a_stable_type_and_payload() {
        let event = DomainEvent::VideoRemovedFromChannel(VideoRemovedFromChannel {
            channel_id: "@somechannel".to_string(),
            video_id: "rec1".to_string(),
            title: "My Video".to_string(),
            filename: Some("My Video.mp4".to_string()),
            thumbnail_filename: Some("My Video.jpg".to_string()),
            was_downloaded: true,
        });

        assert_eq!(event.event_type(), "video_removed_from_channel");
        assert_eq!(
            event.payload(),
            json!({
                "channel_id": "@somechannel",
                "video_id": "rec1",
                "title": "My Video",
                "filename": "My Video.mp4",
                "thumbnail_filename": "My Video.jpg",
                "was_downloaded": true,
            })
        );
    }
}
