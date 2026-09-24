pub mod delete_channel_files_on_channel_deleted;
pub mod delete_playlist_files_on_playlist_deleted;
pub mod delete_video_file_on_video_removed_from_channel;
pub mod delete_video_file_on_video_removed_from_playlist;
pub mod download_video_on_video_added_to_channel;
pub mod download_video_on_video_added_to_playlist;
pub mod reconcile_on_channel_created;
pub mod reconcile_on_playlist_created;

use crate::domain::channel::ChannelVideoReconciler;
use crate::domain::playlist::PlaylistVideoReconciler;
use crate::infrastructure::repositories::domain_events_consumer::SubscriberRegistry;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use delete_channel_files_on_channel_deleted::DeleteChannelFilesOnChannelDeleted;
use delete_playlist_files_on_playlist_deleted::DeletePlaylistFilesOnPlaylistDeleted;
use delete_video_file_on_video_removed_from_channel::DeleteVideoFileOnVideoRemovedFromChannel;
use delete_video_file_on_video_removed_from_playlist::DeleteVideoFileOnVideoRemovedFromPlaylist;
use download_video_on_video_added_to_channel::DownloadVideoOnVideoAddedToChannel;
use download_video_on_video_added_to_playlist::DownloadVideoOnVideoAddedToPlaylist;
use reconcile_on_channel_created::ReconcileOnChannelCreated;
use reconcile_on_playlist_created::ReconcileOnPlaylistCreated;
use std::collections::HashMap;
use std::sync::Arc;

/// Maps each domain event type to the subscribers that react to it, handed
/// to `DomainEventsConsumer` at composition time.
#[allow(clippy::too_many_arguments)]
pub fn registry(
    playlist_video_reconciler: PlaylistVideoReconciler,
    channel_video_reconciler: ChannelVideoReconciler,
    playlist_repository: Arc<dyn PlaylistRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    videos_path: impl Into<String>,
) -> SubscriberRegistry {
    let videos_path = videos_path.into();
    let mut registry: SubscriberRegistry = HashMap::new();
    registry.insert(
        "playlist_created".to_string(),
        vec![Arc::new(ReconcileOnPlaylistCreated::new(
            playlist_video_reconciler,
        ))],
    );
    registry.insert(
        "channel_created".to_string(),
        vec![Arc::new(ReconcileOnChannelCreated::new(
            channel_video_reconciler,
        ))],
    );
    registry.insert(
        "video_added_to_playlist".to_string(),
        vec![Arc::new(DownloadVideoOnVideoAddedToPlaylist::new(
            playlist_repository.clone(),
            task_repository.clone(),
            clock.clone(),
            videos_path.clone(),
        ))],
    );
    registry.insert(
        "video_added_to_channel".to_string(),
        vec![Arc::new(DownloadVideoOnVideoAddedToChannel::new(
            channel_repository.clone(),
            task_repository.clone(),
            clock.clone(),
            videos_path.clone(),
        ))],
    );
    registry.insert(
        "video_removed_from_playlist".to_string(),
        vec![Arc::new(DeleteVideoFileOnVideoRemovedFromPlaylist::new(
            playlist_repository,
            task_repository.clone(),
            clock.clone(),
            videos_path.clone(),
        ))],
    );
    registry.insert(
        "video_removed_from_channel".to_string(),
        vec![Arc::new(DeleteVideoFileOnVideoRemovedFromChannel::new(
            channel_repository,
            task_repository.clone(),
            clock.clone(),
            videos_path,
        ))],
    );
    registry.insert(
        "playlist_deleted".to_string(),
        vec![Arc::new(DeletePlaylistFilesOnPlaylistDeleted::new(
            task_repository.clone(),
            clock.clone(),
        ))],
    );
    registry.insert(
        "channel_deleted".to_string(),
        vec![Arc::new(DeleteChannelFilesOnChannelDeleted::new(
            task_repository,
            clock,
        ))],
    );
    registry
}
