pub mod delete_channel_files_on_channel_deleted;
pub mod delete_playlist_files_on_playlist_deleted;
pub mod delete_video_file_on_video_removed_from_channel;
pub mod delete_video_file_on_video_removed_from_playlist;
pub mod download_video_on_video_added_to_channel;
pub mod download_video_on_video_added_to_playlist;
pub mod fetch_subtitles_on_video_downloaded;
pub mod fetch_thumbnail_on_video_added_to_channel;
pub mod fetch_thumbnail_on_video_added_to_playlist;
pub mod reconcile_on_channel_created;
pub mod reconcile_on_playlist_created;
pub mod scan_plex_folder_on_video_downloaded;
pub mod scan_plex_folder_on_video_subtitles_fetched;

use crate::domain::channel::{ChannelCreated, ChannelDeleted};
use crate::domain::channel_video::{VideoAddedToChannel, VideoRemovedFromChannel};
use crate::domain::playlist::{PlaylistCreated, PlaylistDeleted};
use crate::domain::playlist_video::{VideoAddedToPlaylist, VideoRemovedFromPlaylist};
use crate::domain::services::{ChannelVideoReconciler, PlaylistVideoReconciler, PlexFolderScanner};
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
use download_video_on_video_added_to_channel::DownloadVideoOnVideoAddedToChannel;
use download_video_on_video_added_to_playlist::DownloadVideoOnVideoAddedToPlaylist;
use fetch_subtitles_on_video_downloaded::FetchSubtitlesOnVideoDownloaded;
use fetch_thumbnail_on_video_added_to_channel::FetchThumbnailOnVideoAddedToChannel;
use fetch_thumbnail_on_video_added_to_playlist::FetchThumbnailOnVideoAddedToPlaylist;
use reconcile_on_channel_created::ReconcileOnChannelCreated;
use reconcile_on_playlist_created::ReconcileOnPlaylistCreated;
use scan_plex_folder_on_video_downloaded::ScanPlexFolderOnVideoDownloaded;
use scan_plex_folder_on_video_subtitles_fetched::ScanPlexFolderOnVideoSubtitlesFetched;
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
    plex_folder_scanner: Option<PlexFolderScanner>,
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
    registry.insert(
        VideoAddedToPlaylist::EVENT_TYPE.to_string(),
        vec![
            Arc::new(FetchThumbnailOnVideoAddedToPlaylist::new(
                playlist_repository.clone(),
                task_repository.clone(),
                clock.clone(),
                videos_path.clone(),
            )),
            Arc::new(DownloadVideoOnVideoAddedToPlaylist::new(
                playlist_repository.clone(),
                task_repository.clone(),
                clock.clone(),
                videos_path.clone(),
            )),
        ],
    );
    registry.insert(
        VideoAddedToChannel::EVENT_TYPE.to_string(),
        vec![
            Arc::new(FetchThumbnailOnVideoAddedToChannel::new(
                channel_repository.clone(),
                task_repository.clone(),
                clock.clone(),
                videos_path.clone(),
            )),
            Arc::new(DownloadVideoOnVideoAddedToChannel::new(
                channel_repository.clone(),
                task_repository.clone(),
                clock.clone(),
                videos_path.clone(),
            )),
        ],
    );
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
        vec![Arc::new(DeletePlaylistFilesOnPlaylistDeleted::new(
            task_repository.clone(),
            clock.clone(),
        ))],
    );
    registry.insert(
        ChannelDeleted::EVENT_TYPE.to_string(),
        vec![Arc::new(DeleteChannelFilesOnChannelDeleted::new(
            task_repository.clone(),
            clock.clone(),
        ))],
    );
    registry.insert(
        VideoDownloaded::EVENT_TYPE.to_string(),
        vec![Arc::new(FetchSubtitlesOnVideoDownloaded::new(
            task_repository,
            clock,
        ))],
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
                scanner,
            ))],
        );
    }
    registry
}
