pub mod blocking;
pub mod channels;
pub mod directories;
pub mod error;
pub mod playlists;
pub mod tasks;
pub mod validation;
pub mod videos;

use crate::domain::services::{
    ChannelCreator, ChannelDeleter, ChannelVideoReconciler, ChannelViewSearcher, DirectorySearcher,
    PlaylistCreator, PlaylistDeleter, PlaylistSearcher, PlaylistVideoReconciler, TaskViewSearcher,
    VideoSearcher, VideoWatchStateUpdater,
};
use axum::Router;
use axum::extract::FromRef;
use axum::routing::{get, post};

/// The absolute configured videos root, carried in the adapter layer as the
/// subscribers already do, so no domain type has to know a filesystem
/// location. Reported on every directory listing.
#[derive(Clone)]
pub struct VideosRoot(pub String);

#[derive(Clone, FromRef)]
pub struct ApiServices {
    pub playlist_creator: PlaylistCreator,
    pub playlist_deleter: PlaylistDeleter,
    pub playlist_searcher: PlaylistSearcher,
    pub playlist_video_reconciler: PlaylistVideoReconciler,
    pub video_searcher: VideoSearcher,
    pub task_view_searcher: TaskViewSearcher,
    pub channel_creator: ChannelCreator,
    pub channel_deleter: ChannelDeleter,
    pub channel_view_searcher: ChannelViewSearcher,
    pub channel_video_reconciler: ChannelVideoReconciler,
    pub directory_searcher: DirectorySearcher,
    pub video_watch_state_updater: VideoWatchStateUpdater,
    pub videos_root: VideosRoot,
}

pub fn api_router(api_services: ApiServices) -> Router {
    Router::new()
        .route("/directories", get(directories::list_directories))
        .route(
            "/playlists",
            post(playlists::create_playlist).get(playlists::list_playlists),
        )
        .route(
            "/playlists/{id}",
            axum::routing::delete(playlists::delete_playlist),
        )
        .route(
            "/playlists/{id}/reconcile",
            post(playlists::reconcile_playlist),
        )
        .route(
            "/playlists/{id}/videos",
            get(videos::list_videos_for_playlist),
        )
        .route("/videos/recent", get(videos::list_recent_videos))
        .route("/videos/{id}/progress", post(videos::record_video_progress))
        .route("/tasks", get(tasks::list_tasks))
        .route(
            "/channels",
            post(channels::create_channel).get(channels::list_channels),
        )
        .route(
            "/channels/{handle}",
            axum::routing::delete(channels::delete_channel),
        )
        .route(
            "/channels/{handle}/reconcile",
            post(channels::reconcile_channel),
        )
        .route(
            "/channels/{handle}/videos",
            get(videos::list_videos_for_channel),
        )
        .route(
            "/channels/{handle}/watched",
            post(channels::mark_channel_watched),
        )
        .with_state(api_services)
}
