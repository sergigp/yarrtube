pub mod sync_playlist_on_playlist_created;

use crate::domain::video::VideoService;
use crate::infrastructure::repositories::domain_events_consumer::SubscriberRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use sync_playlist_on_playlist_created::SyncPlaylistOnPlaylistCreated;

/// Maps each domain event type to the subscribers that react to it, handed
/// to `DomainEventsConsumer` at composition time.
pub fn registry(video_service: VideoService) -> SubscriberRegistry {
    let mut registry: SubscriberRegistry = HashMap::new();
    registry.insert(
        "playlist_created".to_string(),
        vec![Arc::new(SyncPlaylistOnPlaylistCreated::new(video_service))],
    );
    registry
}
