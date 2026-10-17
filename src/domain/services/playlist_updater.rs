use crate::domain::playlist::Playlist;
use crate::domain::playlist::PlaylistId;
use crate::domain::playlist::errors::UpdatePlaylistError;
use crate::domain::shared::ReconcileInterval;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;
use tracing::info;

/// Changes the settings of an existing playlist.
#[derive(Clone)]
pub struct PlaylistUpdater {
    repository: Arc<dyn PlaylistRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl PlaylistUpdater {
    pub fn new(
        repository: Arc<dyn PlaylistRepository>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repository,
            task_repository,
            clock,
        }
    }
}

pub trait PlaylistUpdaterApi: Send + Sync {
    /// Sets whether the playlist's videos are left out of home and/or its
    /// reconcile interval, returning the playlist as stored. `None` keeps the
    /// stored value; setting the value it already has changes nothing. A new
    /// interval brings the pending reconcile forward when it would now run
    /// later than `now + interval`.
    fn update_settings(
        &self,
        id: PlaylistId,
        exclude_from_home: Option<bool>,
        reconcile_interval: Option<ReconcileInterval>,
    ) -> Result<Playlist, UpdatePlaylistError>;
}

impl PlaylistUpdaterApi for PlaylistUpdater {
    fn update_settings(
        &self,
        id: PlaylistId,
        exclude_from_home: Option<bool>,
        _reconcile_interval: Option<ReconcileInterval>,
    ) -> Result<Playlist, UpdatePlaylistError> {
        let playlist = Self::apply_settings(self.find_playlist(&id)?, exclude_from_home);
        self.update_playlist(&playlist)?;
        info!(
            playlist_id = %id,
            exclude_from_home = playlist.exclude_from_home,
            "updated playlist"
        );
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

    fn apply_settings(playlist: Playlist, exclude_from_home: Option<bool>) -> Playlist {
        match exclude_from_home {
            Some(exclude_from_home) => playlist.with_exclude_from_home(exclude_from_home),
            None => playlist,
        }
    }

    fn update_playlist(&self, playlist: &Playlist) -> Result<(), UpdatePlaylistError> {
        self.repository
            .update(playlist)
            .map_err(UpdatePlaylistError::Repository)
    }
}
