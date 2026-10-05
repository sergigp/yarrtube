use crate::domain::event::DomainEvent;
use crate::domain::playlist::PlaylistCreated;
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
        path: PlaylistPath,
        quality: Quality,
        exclude_from_home: bool,
    ) -> Result<CreatePlaylistOutcome, CreatePlaylistError>;
}

impl PlaylistCreatorApi for PlaylistCreator {
    fn create(
        &self,
        id: PlaylistId,
        path: PlaylistPath,
        quality: Quality,
        _exclude_from_home: bool,
    ) -> Result<CreatePlaylistOutcome, CreatePlaylistError> {
        if let Some(existing) = self.find_existing(&id)? {
            return Ok(CreatePlaylistOutcome::AlreadyExisted(existing));
        }

        self.ensure_path_free(&path)?;
        let name = self.resolve_on_youtube(&id)?;
        let now = self.clock.now();
        let playlist = Playlist::create(
            id,
            name,
            path,
            quality,
            PlaylistKind::YoutubeLinked,
            false,
            now,
        );
        self.insert_playlist(&playlist)?;
        self.publish_playlist_created(&playlist)?;
        info!(playlist_id = %playlist.id, name = %playlist.name, "created playlist");
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

    fn resolve_on_youtube(&self, id: &PlaylistId) -> Result<PlaylistName, CreatePlaylistError> {
        match self.lookup.resolve(id) {
            Ok(Some(resolved)) => Ok(PlaylistName::from_youtube_title(&resolved.title, id)),
            Ok(None) => Err(CreatePlaylistError::YoutubePlaylistNotFound(id.clone())),
            Err(e) => Err(CreatePlaylistError::Lookup(e)),
        }
    }

    fn insert_playlist(&self, playlist: &Playlist) -> Result<(), CreatePlaylistError> {
        self.repository
            .insert(playlist)
            .map_err(CreatePlaylistError::Repository)
    }

    fn publish_playlist_created(&self, playlist: &Playlist) -> Result<(), CreatePlaylistError> {
        self.event_publisher
            .publish(&DomainEvent::PlaylistCreated(PlaylistCreated {
                playlist_id: playlist.id.as_str().to_string(),
            }))
            .map_err(CreatePlaylistError::Repository)
    }
}
