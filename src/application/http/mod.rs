pub mod channels;
pub mod custom_playlists;
pub mod error;
pub mod playlists;
pub mod tasks;
pub mod videos;

use crate::domain::channel::ChannelService;
use crate::domain::services::{
    ChannelVideoReconciler, CustomPlaylistVideoAdder, CustomPlaylistVideoRemover, PlaylistCreator,
    PlaylistDeleter, PlaylistSearcher, VideoReconciler, VideoSearcher,
};
use crate::domain::task::TaskService;
use axum::Router;
use axum::routing::{get, post};

#[derive(Clone)]
pub struct AppState {
    pub playlist_creator: PlaylistCreator,
    pub playlist_deleter: PlaylistDeleter,
    pub playlist_searcher: PlaylistSearcher,
    pub video_reconciler: VideoReconciler,
    pub custom_playlist_video_adder: CustomPlaylistVideoAdder,
    pub custom_playlist_video_remover: CustomPlaylistVideoRemover,
    pub video_searcher: VideoSearcher,
    pub task_service: TaskService,
    pub channel_service: ChannelService,
    pub channel_video_reconciler: ChannelVideoReconciler,
}

pub fn api_router(state: AppState) -> Router {
    Router::new()
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
        .route(
            "/custom-playlists",
            post(custom_playlists::create_custom_playlist),
        )
        .route(
            "/custom-playlists/{id}/videos",
            post(custom_playlists::add_video),
        )
        .route(
            "/custom-playlists/{id}/videos/{video_id}",
            axum::routing::delete(custom_playlists::remove_video),
        )
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
