use crate::domain::playlist::Playlist;
use crate::domain::playlist::PlaylistId;
use crate::domain::playlist::errors::UpdatePlaylistError;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use std::sync::Arc;
use tracing::info;

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
        exclude_from_home: bool,
    ) -> Result<Playlist, UpdatePlaylistError> {
        let playlist = self
            .find_playlist(&id)?
            .with_exclude_from_home(exclude_from_home);
        self.update_playlist(&playlist)?;
        info!(playlist_id = %id, exclude_from_home, "updated playlist");
        Ok(playlist)
    }
}

impl PlaylistUpdater {
    fn find_playlist(&self, id: &PlaylistId) -> Result<Playlist, UpdatePlaylistError> {
        match self.repository.find(id) {
            Ok(Some(playlist)) => Ok(playlist),
            Ok(None) => Err(UpdatePlaylistError::NotFound(id.clone())),
            Err(e) => Err(UpdatePlaylistError::Repository(e)),
        }
    }

    fn update_playlist(&self, playlist: &Playlist) -> Result<(), UpdatePlaylistError> {
        self.repository
            .update(playlist)
            .map_err(UpdatePlaylistError::Repository)
    }
}
