use crate::domain::playlist::Playlist;
use crate::domain::shared::PlaylistId;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use std::sync::Arc;

/// Reads playlists, single or all.
#[derive(Clone)]
pub struct PlaylistSearcher {
    repository: Arc<dyn PlaylistRepository>,
}

impl PlaylistSearcher {
    pub fn new(repository: Arc<dyn PlaylistRepository>) -> Self {
        Self { repository }
    }

    pub fn search(&self, id: &PlaylistId) -> anyhow::Result<Option<Playlist>> {
        self.repository.find(id)
    }

    pub fn search_all(&self) -> anyhow::Result<Vec<Playlist>> {
        self.repository.list()
    }
}
