pub mod custom_playlists;
pub mod error;
pub mod playlists;
pub mod tasks;
pub mod videos;

use crate::domain::playlist::PlaylistService;
use crate::domain::task::TaskService;
use crate::domain::video::VideoService;
use axum::Router;
use axum::routing::{get, post};

#[derive(Clone)]
pub struct AppState {
    pub playlist_service: PlaylistService,
    pub video_service: VideoService,
    pub task_service: TaskService,
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
        .with_state(state)
}
