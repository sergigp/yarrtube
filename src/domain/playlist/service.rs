use super::errors::{CreatePlaylistError, DeletePlaylistError};
use super::playlist::Playlist;
use super::playlist_name::PlaylistName;
use super::quality::Quality;
use crate::domain::event::DomainEvent;
use crate::domain::shared::PlaylistId;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::youtube_playlist_repository::YoutubePlaylistRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreatePlaylistOutcome {
    Created(Playlist),
    AlreadyExisted(Playlist),
}

/// Orchestrates every operation on the playlist aggregate. Injected with only the
/// ports playlist operations actually use — not every port the application has.
#[derive(Clone)]
pub struct PlaylistService {
    repository: Arc<dyn PlaylistRepository>,
    lookup: Arc<dyn YoutubePlaylistRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
}

impl PlaylistService {
    pub fn new(
        repository: Arc<dyn PlaylistRepository>,
        lookup: Arc<dyn YoutubePlaylistRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repository,
            lookup,
            event_publisher,
            clock,
        }
    }

    pub fn create_playlist(
        &self,
        id: PlaylistId,
        name: PlaylistName,
        quality: Quality,
    ) -> Result<CreatePlaylistOutcome, CreatePlaylistError> {
        match self.lookup.exists(&id) {
            Ok(true) => {}
            Ok(false) => return Err(CreatePlaylistError::YoutubePlaylistNotFound(id)),
            Err(e) => return Err(CreatePlaylistError::Lookup(e)),
        }

        match self.repository.find(&id) {
            Ok(Some(existing)) => return Ok(CreatePlaylistOutcome::AlreadyExisted(existing)),
            Ok(None) => {}
            Err(e) => return Err(CreatePlaylistError::Repository(e)),
        }

        let now = self.clock.now();
        let playlist = Playlist::create(id, name, quality, now);
        self.repository
            .insert(&playlist)
            .map_err(CreatePlaylistError::Repository)?;
        self.event_publisher
            .publish(&DomainEvent::PlaylistCreated {
                playlist_id: playlist.id.as_str().to_string(),
            })
            .map_err(CreatePlaylistError::Repository)?;
        info!(playlist_id = %playlist.id, name = %playlist.name, "created playlist");
        Ok(CreatePlaylistOutcome::Created(playlist))
    }

    pub fn delete_playlist(&self, id: PlaylistId) -> Result<(), DeletePlaylistError> {
        match self.repository.find(&id) {
            Ok(Some(_)) => {}
            Ok(None) => return Err(DeletePlaylistError::NotFound(id)),
            Err(e) => return Err(DeletePlaylistError::Repository(e)),
        }

        self.repository
            .delete(&id)
            .map_err(DeletePlaylistError::Repository)?;
        self.event_publisher
            .publish(&DomainEvent::PlaylistDeleted {
                playlist_id: id.as_str().to_string(),
            })
            .map_err(DeletePlaylistError::Repository)?;
        info!(playlist_id = %id, "deleted playlist");
        Ok(())
    }

    pub fn list_playlists(&self) -> anyhow::Result<Vec<Playlist>> {
        self.repository.list()
    }
}
