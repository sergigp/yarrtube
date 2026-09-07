pub mod error;
pub mod playlists;

use crate::domain::playlist::PlaylistService;
use axum::Router;
use axum::routing::post;

#[derive(Clone)]
pub struct AppState {
    pub playlist_service: PlaylistService,
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
        .with_state(state)
}
