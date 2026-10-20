use crate::domain::playlist::Playlist;
use crate::domain::playlist::PlaylistId;
use crate::domain::playlist::PlaylistSettingsChange;
use crate::domain::playlist::errors::UpdatePlaylistError;
use crate::domain::shared::ReconcileInterval;
use crate::domain::task::Task;
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
    /// Applies `change` (whether the playlist's videos are left out of home,
    /// whether its unwatched count is hidden, its reconcile interval),
    /// returning the playlist as stored. A `None` setting keeps the stored
    /// value; setting the value it already has changes nothing. A new
    /// interval brings the pending reconcile forward when it would now run
    /// later than `now + interval`.
    fn update_settings(
        &self,
        id: PlaylistId,
        change: PlaylistSettingsChange,
    ) -> Result<Playlist, UpdatePlaylistError>;
}

impl PlaylistUpdaterApi for PlaylistUpdater {
    fn update_settings(
        &self,
        id: PlaylistId,
        change: PlaylistSettingsChange,
    ) -> Result<Playlist, UpdatePlaylistError> {
        let playlist = Self::apply_settings(self.find_playlist(&id)?, change);
        self.update_playlist(&playlist)?;
        if let Some(reconcile_interval) = change.reconcile_interval {
            self.bring_forward_next_reconcile(&id, reconcile_interval)?;
        }
        info!(
            playlist_id = %id,
            exclude_from_home = playlist.exclude_from_home,
            hide_unwatched_count = playlist.hide_unwatched_count,
            reconcile_interval_minutes = playlist.reconcile_interval.minutes(),
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

    fn apply_settings(playlist: Playlist, change: PlaylistSettingsChange) -> Playlist {
        let playlist = match change.exclude_from_home {
            Some(exclude_from_home) => playlist.with_exclude_from_home(exclude_from_home),
            None => playlist,
        };
        let playlist = match change.hide_unwatched_count {
            Some(hide_unwatched_count) => playlist.with_hide_unwatched_count(hide_unwatched_count),
            None => playlist,
        };
        match change.reconcile_interval {
            Some(reconcile_interval) => playlist.with_reconcile_interval(reconcile_interval),
            None => playlist,
        }
    }

    fn update_playlist(&self, playlist: &Playlist) -> Result<(), UpdatePlaylistError> {
        self.repository
            .update(playlist)
            .map_err(UpdatePlaylistError::Repository)
    }

    /// A shorter interval applies right away: the pending reconcile moves to
    /// `now + interval` when it was due later.
    fn bring_forward_next_reconcile(
        &self,
        id: &PlaylistId,
        reconcile_interval: ReconcileInterval,
    ) -> Result<(), UpdatePlaylistError> {
        let task = Task::ReconcilePlaylist {
            playlist_id: id.as_str().to_string(),
        };
        let Some(pending) = self
            .task_repository
            .find_pending(&task)
            .map_err(UpdatePlaylistError::Repository)?
        else {
            return Ok(());
        };
        let now = self.clock.now();
        self.task_repository
            .update(&pending.bring_forward(now + reconcile_interval.duration(), now))
            .map_err(UpdatePlaylistError::Repository)
    }
}
