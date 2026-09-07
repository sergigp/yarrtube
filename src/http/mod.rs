pub mod playlists;

use crate::domain::ports::{Clock, PlaylistRepository, YoutubePlaylistLookup};
use axum::Router;
use axum::routing::post;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub repository: Arc<dyn PlaylistRepository>,
    pub lookup: Arc<dyn YoutubePlaylistLookup>,
    pub clock: Arc<dyn Clock>,
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
