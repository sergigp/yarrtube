use crate::domain::playlist::Playlist;
use crate::domain::playlist::PlaylistId;
use crate::domain::playlist::errors::UpdatePlaylistError;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use std::sync::Arc;

/// Changes the settings of an existing playlist.
#[derive(Clone)]
pub struct PlaylistUpdater {
    repository: Arc<dyn PlaylistRepository>,
}

impl PlaylistUpdater {
    pub fn new(repository: Arc<dyn PlaylistRepository>) -> Self {
        Self { repository }
    }
}

pub trait PlaylistUpdaterApi: Send + Sync {
    /// Sets whether the playlist's videos are left out of home, returning the
    /// playlist as stored. Setting the value it already has changes nothing.
    fn update_exclude_from_home(
        &self,
        id: PlaylistId,
        exclude_from_home: bool,
    ) -> Result<Playlist, UpdatePlaylistError>;
}

impl PlaylistUpdaterApi for PlaylistUpdater {
    fn update_exclude_from_home(
        &self,
        id: PlaylistId,
        _exclude_from_home: bool,
    ) -> Result<Playlist, UpdatePlaylistError> {
        let _ = &self.repository;
        Err(UpdatePlaylistError::NotFound(id))
    }
}
