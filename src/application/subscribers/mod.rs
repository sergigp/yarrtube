pub mod delete_channel_files_on_channel_deleted;
pub mod delete_playlist_files_on_playlist_deleted;
pub mod delete_video_file_on_video_removed_from_channel;
pub mod delete_video_file_on_video_removed_from_playlist;
pub mod reconcile_on_channel_created;
pub mod reconcile_on_playlist_created;
pub mod remove_channel_from_categories_on_channel_deleted;
pub mod remove_playlist_from_categories_on_playlist_deleted;
pub mod scan_plex_folder_on_show_moved;
pub mod scan_plex_folder_on_video_downloaded;
pub mod scan_plex_folder_on_video_subtitles_fetched;
pub mod sync_video_on_video_event;

use crate::domain::channel::{ChannelCreated, ChannelDeleted};
use crate::domain::channel_video::{VideoAddedToChannel, VideoRemovedFromChannel};
use crate::domain::playlist::{PlaylistCreated, PlaylistDeleted};
use crate::domain::playlist_video::{VideoAddedToPlaylist, VideoRemovedFromPlaylist};
use crate::domain::services::{
    CategoryMemberRemover, ChannelVideoReconciler, PlaylistVideoReconciler, PlexFolderScanner,
};
use crate::domain::show::ShowMoved;
use crate::domain::video::{VideoDownloaded, VideoSubtitlesFetched};
use crate::infrastructure::repositories::domain_events_consumer::SubscriberRegistry;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use delete_channel_files_on_channel_deleted::DeleteChannelFilesOnChannelDeleted;
use delete_playlist_files_on_playlist_deleted::DeletePlaylistFilesOnPlaylistDeleted;
use delete_video_file_on_video_removed_from_channel::DeleteVideoFileOnVideoRemovedFromChannel;
use delete_video_file_on_video_removed_from_playlist::DeleteVideoFileOnVideoRemovedFromPlaylist;
use reconcile_on_channel_created::ReconcileOnChannelCreated;
use reconcile_on_playlist_created::ReconcileOnPlaylistCreated;
use remove_channel_from_categories_on_channel_deleted::RemoveChannelFromCategoriesOnChannelDeleted;
use remove_playlist_from_categories_on_playlist_deleted::RemovePlaylistFromCategoriesOnPlaylistDeleted;
use scan_plex_folder_on_show_moved::ScanPlexFolderOnShowMoved;
use scan_plex_folder_on_video_downloaded::ScanPlexFolderOnVideoDownloaded;
use scan_plex_folder_on_video_subtitles_fetched::ScanPlexFolderOnVideoSubtitlesFetched;
use std::collections::HashMap;
use std::sync::Arc;
use sync_video_on_video_event::SyncVideoOnVideoEvent;

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
    plex_folder_scanner: Option<PlexFolderScanner>,
    category_member_remover: CategoryMemberRemover,
) -> SubscriberRegistry {
    let videos_path = videos_path.into();
    let mut registry: SubscriberRegistry = HashMap::new();
    registry.insert(
        PlaylistCreated::EVENT_TYPE.to_string(),
        vec![Arc::new(ReconcileOnPlaylistCreated::new(
            playlist_video_reconciler,
        ))],
    );
    registry.insert(
        ChannelCreated::EVENT_TYPE.to_string(),
        vec![Arc::new(ReconcileOnChannelCreated::new(
            channel_video_reconciler,
        ))],
    );
    [
        VideoAddedToPlaylist::EVENT_TYPE,
        VideoAddedToChannel::EVENT_TYPE,
        VideoDownloaded::EVENT_TYPE,
    ]
    .into_iter()
    .for_each(|event_type| {
        registry.insert(
            event_type.to_string(),
            vec![Arc::new(SyncVideoOnVideoEvent::new(
                task_repository.clone(),
                clock.clone(),
            ))],
        );
    });
    registry.insert(
        VideoRemovedFromPlaylist::EVENT_TYPE.to_string(),
        vec![Arc::new(DeleteVideoFileOnVideoRemovedFromPlaylist::new(
            playlist_repository,
            task_repository.clone(),
            clock.clone(),
            videos_path.clone(),
        ))],
    );
    registry.insert(
        VideoRemovedFromChannel::EVENT_TYPE.to_string(),
        vec![Arc::new(DeleteVideoFileOnVideoRemovedFromChannel::new(
            channel_repository,
            task_repository.clone(),
            clock.clone(),
            videos_path,
        ))],
    );
    registry.insert(
        PlaylistDeleted::EVENT_TYPE.to_string(),
        vec![
            Arc::new(DeletePlaylistFilesOnPlaylistDeleted::new(
                task_repository.clone(),
                clock.clone(),
            )),
            Arc::new(RemovePlaylistFromCategoriesOnPlaylistDeleted::new(
                category_member_remover.clone(),
            )),
        ],
    );
    registry.insert(
        ChannelDeleted::EVENT_TYPE.to_string(),
        vec![
            Arc::new(DeleteChannelFilesOnChannelDeleted::new(
                task_repository.clone(),
                clock.clone(),
            )),
            Arc::new(RemoveChannelFromCategoriesOnChannelDeleted::new(
                category_member_remover,
            )),
        ],
    );
    if let Some(scanner) = plex_folder_scanner {
        registry
            .entry(VideoDownloaded::EVENT_TYPE.to_string())
            .or_default()
            .push(Arc::new(ScanPlexFolderOnVideoDownloaded::new(
                scanner.clone(),
            )));
        registry.insert(
            VideoSubtitlesFetched::EVENT_TYPE.to_string(),
            vec![Arc::new(ScanPlexFolderOnVideoSubtitlesFetched::new(
                scanner.clone(),
            ))],
        );
        registry.insert(
            ShowMoved::EVENT_TYPE.to_string(),
            vec![Arc::new(ScanPlexFolderOnShowMoved::new(scanner))],
        );
    }
    registry
}
