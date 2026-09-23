pub mod channels;
pub mod directories;
pub mod error;
pub mod playlists;
pub mod tasks;
#[cfg(test)]
pub mod test_support;
pub mod videos;

use crate::domain::channel::ChannelService;
use crate::domain::services::{
    ChannelVideoReconciler, DirectorySearcher, PlaylistCreator, PlaylistDeleter, PlaylistSearcher,
    TaskViewSearcher, VideoReconciler, VideoSearcher,
};
use axum::Router;
use axum::routing::{get, post};

#[derive(Clone)]
pub struct AppState {
    pub playlist_creator: PlaylistCreator,
    pub playlist_deleter: PlaylistDeleter,
    pub playlist_searcher: PlaylistSearcher,
    pub video_reconciler: VideoReconciler,
    pub video_searcher: VideoSearcher,
    pub task_view_searcher: TaskViewSearcher,
    pub channel_service: ChannelService,
    pub channel_video_reconciler: ChannelVideoReconciler,
    pub directory_searcher: DirectorySearcher,
    /// The absolute configured videos root, carried in the adapter layer as
    /// the subscribers already do, so no domain type has to know a filesystem
    /// location. Reported on every directory listing.
    pub videos_root: String,
}

pub fn api_router(state: AppState) -> Router {
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
        .with_state(state)
}
