use crate::domain::event::DomainEvent;
use crate::domain::playlist::PlaylistCreated;
use crate::domain::playlist::PlaylistId;
use crate::domain::playlist::errors::CreatePlaylistError;
use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
use crate::domain::shared::{Quality, ReconcileInterval};
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
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
    preferences_repository: Arc<dyn PreferencesRepository>,
}

impl PlaylistCreator {
    pub fn new(
        repository: Arc<dyn PlaylistRepository>,
        lookup: Arc<dyn YoutubePlaylistRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
        preferences_repository: Arc<dyn PreferencesRepository>,
    ) -> Self {
        Self {
            repository,
            lookup,
            event_publisher,
            clock,
            preferences_repository,
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
        reconcile_interval: ReconcileInterval,
        summaries_enabled: bool,
    ) -> Result<CreatePlaylistOutcome, CreatePlaylistError>;
}

impl PlaylistCreatorApi for PlaylistCreator {
    fn create(
        &self,
        id: PlaylistId,
        path: PlaylistPath,
        quality: Quality,
        exclude_from_home: bool,
        reconcile_interval: ReconcileInterval,
        summaries_enabled: bool,
    ) -> Result<CreatePlaylistOutcome, CreatePlaylistError> {
        self.ensure_summaries_available(summaries_enabled)?;
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
            exclude_from_home,
            reconcile_interval,
            now,
        )
        .with_summaries_enabled(summaries_enabled);
        self.insert_playlist(&playlist)?;
        self.publish_playlist_created(&playlist)?;
        info!(playlist_id = %playlist.id, name = %playlist.name, "created playlist");
        Ok(CreatePlaylistOutcome::Created(playlist))
    }
}

impl PlaylistCreator {
    /// Enabling summaries needs AI and summaries on in the preferences.
    fn ensure_summaries_available(
        &self,
        summaries_enabled: bool,
    ) -> Result<(), CreatePlaylistError> {
        if !summaries_enabled {
            return Ok(());
        }
        let preferences = self
            .preferences_repository
            .find()
            .map_err(CreatePlaylistError::Repository)?
            .unwrap_or_default();
        if preferences.summaries_available() {
            Ok(())
        } else {
            Err(CreatePlaylistError::SummariesUnavailable)
        }
    }

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
