pub mod delete_video_file_on_video_deleted;
pub mod download_video_on_video_added;
pub mod sync_playlist_on_playlist_created;

use crate::domain::video::VideoService;
use crate::infrastructure::repositories::domain_events_consumer::SubscriberRegistry;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::system_clock::Clock;
use delete_video_file_on_video_deleted::DeleteVideoFileOnVideoDeleted;
use download_video_on_video_added::DownloadVideoOnVideoAdded;
use std::collections::HashMap;
use std::sync::Arc;
use sync_playlist_on_playlist_created::SyncPlaylistOnPlaylistCreated;

/// Maps each domain event type to the subscribers that react to it, handed
/// to `DomainEventsConsumer` at composition time.
pub fn registry(
    video_service: VideoService,
    playlist_repository: Arc<dyn PlaylistRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
) -> SubscriberRegistry {
    let mut registry: SubscriberRegistry = HashMap::new();
    registry.insert(
        "playlist_created".to_string(),
        vec![Arc::new(SyncPlaylistOnPlaylistCreated::new(video_service))],
    );
    registry.insert(
        "video_added".to_string(),
        vec![Arc::new(DownloadVideoOnVideoAdded::new(
            playlist_repository,
            task_repository.clone(),
            clock.clone(),
        ))],
    );
    registry.insert(
        "video_deleted".to_string(),
        vec![Arc::new(DeleteVideoFileOnVideoDeleted::new(
            task_repository,
            clock,
        ))],
    );
    registry
}
