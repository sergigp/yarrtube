pub mod custom_playlists;
pub mod error;
pub mod playlists;

use crate::domain::playlist::PlaylistService;
use crate::domain::video::VideoService;
use axum::Router;
use axum::routing::post;

#[derive(Clone)]
pub struct AppState {
    pub playlist_service: PlaylistService,
    pub video_service: VideoService,
}

pub fn playlists_router(state: AppState) -> Router {
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
        .with_state(state)
}
