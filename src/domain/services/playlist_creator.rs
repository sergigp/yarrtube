use crate::domain::event::DomainEvent;
use crate::domain::playlist::PlaylistId;
use crate::domain::playlist::errors::CreatePlaylistError;
use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
use crate::domain::shared::Quality;
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

/// Creates playlists backed by a YouTube playlist.
#[derive(Clone)]
pub struct PlaylistCreator {
    repository: Arc<dyn PlaylistRepository>,
    lookup: Arc<dyn YoutubePlaylistRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
}

impl PlaylistCreator {
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
}

pub trait PlaylistCreatorApi: Send + Sync {
    fn create(
        &self,
        id: PlaylistId,
        name: PlaylistName,
        path: PlaylistPath,
        quality: Quality,
    ) -> Result<CreatePlaylistOutcome, CreatePlaylistError>;
}

impl PlaylistCreatorApi for PlaylistCreator {
    fn create(
        &self,
        id: PlaylistId,
        name: PlaylistName,
        path: PlaylistPath,
        quality: Quality,
    ) -> Result<CreatePlaylistOutcome, CreatePlaylistError> {
        if let Some(existing) = self.find_existing(&id)? {
            return Ok(CreatePlaylistOutcome::AlreadyExisted(existing));
        }

        self.ensure_path_free(&path)?;
        self.ensure_exists_on_youtube(&id)?;
        let now = self.clock.now();
        let playlist = Playlist::create(id, name, path, quality, PlaylistKind::YoutubeLinked, now);
        self.insert_and_publish(&playlist)?;
        Ok(CreatePlaylistOutcome::Created(playlist))
    }
}

impl PlaylistCreator {
    fn find_existing(&self, id: &PlaylistId) -> Result<Option<Playlist>, CreatePlaylistError> {
        self.repository
            .find(id)
            .map_err(CreatePlaylistError::Repository)
    }

    fn ensure_path_free(&self, path: &PlaylistPath) -> Result<(), CreatePlaylistError> {
        let playlists = self
            .repository
            .list()
            .map_err(CreatePlaylistError::Repository)?;
        if playlists.iter().any(|playlist| &playlist.path == path) {
            return Err(CreatePlaylistError::PathAlreadyInUse(path.clone()));
        }
        Ok(())
    }

    fn ensure_exists_on_youtube(&self, id: &PlaylistId) -> Result<(), CreatePlaylistError> {
        match self.lookup.exists(id) {
            Ok(true) => Ok(()),
            Ok(false) => Err(CreatePlaylistError::YoutubePlaylistNotFound(id.clone())),
            Err(e) => Err(CreatePlaylistError::Lookup(e)),
        }
    }

    fn insert_and_publish(&self, playlist: &Playlist) -> Result<(), CreatePlaylistError> {
        self.repository
            .insert(playlist)
            .map_err(CreatePlaylistError::Repository)?;
        self.event_publisher
            .publish(&DomainEvent::PlaylistCreated {
                playlist_id: playlist.id.as_str().to_string(),
            })
            .map_err(CreatePlaylistError::Repository)?;
        info!(playlist_id = %playlist.id, name = %playlist.name, "created playlist");
        Ok(())
    }
}
