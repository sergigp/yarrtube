use crate::domain::event::DomainEvent;
use crate::domain::playlist::errors::{CreateCustomPlaylistError, CreatePlaylistError};
use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
use crate::domain::shared::{PlaylistId, Quality};
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::youtube_playlist_repository::YoutubePlaylistRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreatePlaylistOutcome {
    Created(Playlist),
    AlreadyExisted(Playlist),
}

/// Creates playlists, either backed by a YouTube playlist or as a custom
/// (YouTube-less) playlist.
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

    pub fn create(
        &self,
        id: PlaylistId,
        name: PlaylistName,
        path: PlaylistPath,
        quality: Quality,
    ) -> Result<CreatePlaylistOutcome, CreatePlaylistError> {
        match self.repository.find(&id) {
            Ok(Some(existing)) => return Ok(CreatePlaylistOutcome::AlreadyExisted(existing)),
            Ok(None) => {}
            Err(e) => return Err(CreatePlaylistError::Repository(e)),
        }

        let playlists = self
            .repository
            .list()
            .map_err(CreatePlaylistError::Repository)?;
        if path_used_by_another_playlist(&playlists, &id, &path) {
            return Err(CreatePlaylistError::PathAlreadyInUse(path));
        }

        match self.lookup.exists(&id) {
            Ok(true) => {}
            Ok(false) => return Err(CreatePlaylistError::YoutubePlaylistNotFound(id)),
            Err(e) => return Err(CreatePlaylistError::Lookup(e)),
        }

        let now = self.clock.now();
        let playlist = Playlist::create(id, name, path, quality, PlaylistKind::YoutubeLinked, now);
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

    /// Creates a playlist with no YouTube playlist behind it: the caller
    /// supplies the ID (a well-formed, unused UUID) directly, and no YouTube
    /// API request is made.
    pub fn create_custom(
        &self,
        id: PlaylistId,
        name: PlaylistName,
        path: PlaylistPath,
        quality: Quality,
    ) -> Result<Playlist, CreateCustomPlaylistError> {
        if let Err(e) = Uuid::parse_str(id.as_str()) {
            return Err(CreateCustomPlaylistError::InvalidId(id, e.to_string()));
        }

        match self.repository.find(&id) {
            Ok(Some(_)) => return Err(CreateCustomPlaylistError::AlreadyExists(id)),
            Ok(None) => {}
            Err(e) => return Err(CreateCustomPlaylistError::Repository(e)),
        }

        let playlists = self
            .repository
            .list()
            .map_err(CreateCustomPlaylistError::Repository)?;
        if path_used_by_another_playlist(&playlists, &id, &path) {
            return Err(CreateCustomPlaylistError::PathAlreadyInUse(path));
        }

        let now = self.clock.now();
        let playlist = Playlist::create(id, name, path, quality, PlaylistKind::Custom, now);
        self.repository
            .insert(&playlist)
            .map_err(CreateCustomPlaylistError::Repository)?;
        self.event_publisher
            .publish(&DomainEvent::PlaylistCreated {
                playlist_id: playlist.id.as_str().to_string(),
            })
            .map_err(CreateCustomPlaylistError::Repository)?;
        info!(playlist_id = %playlist.id, name = %playlist.name, "created custom playlist");
        Ok(playlist)
    }
}

/// True if some playlist other than `id` already has `path` as its stored
/// path. Used to reject a create request before it can produce two
/// playlists sharing an output directory.
fn path_used_by_another_playlist(
    playlists: &[Playlist],
    id: &PlaylistId,
    path: &PlaylistPath,
) -> bool {
    playlists.iter().any(|p| p.id != *id && p.path == *path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::playlist::PlaylistKind;
    use chrono::{DateTime, Utc};

    fn playlist(id: &str, path: &str, kind: PlaylistKind) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new("Playlist").unwrap(),
            PlaylistPath::new(path).unwrap(),
            Quality::High,
            kind,
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
    }

    #[test]
    fn it_should_be_false_when_no_other_playlist_uses_the_path() {
        let playlists = vec![playlist("PL1", "music", PlaylistKind::YoutubeLinked)];

        let used = path_used_by_another_playlist(
            &playlists,
            &PlaylistId::new("PL2").unwrap(),
            &PlaylistPath::new("videos").unwrap(),
        );

        assert!(!used);
    }

    #[test]
    fn it_should_be_true_when_a_different_playlist_already_uses_the_path() {
        let playlists = vec![playlist("PL1", "music", PlaylistKind::YoutubeLinked)];

        let used = path_used_by_another_playlist(
            &playlists,
            &PlaylistId::new("PL2").unwrap(),
            &PlaylistPath::new("music").unwrap(),
        );

        assert!(used);
    }

    #[test]
    fn it_should_be_true_when_the_colliding_playlist_is_of_the_other_kind() {
        let playlists = vec![playlist("PL1", "music", PlaylistKind::Custom)];

        let used = path_used_by_another_playlist(
            &playlists,
            &PlaylistId::new("PL2").unwrap(),
            &PlaylistPath::new("music").unwrap(),
        );

        assert!(used);
    }

    #[test]
    fn it_should_be_false_when_the_path_belongs_to_the_playlist_itself() {
        let playlists = vec![playlist("PL1", "music", PlaylistKind::YoutubeLinked)];

        let used = path_used_by_another_playlist(
            &playlists,
            &PlaylistId::new("PL1").unwrap(),
            &PlaylistPath::new("music").unwrap(),
        );

        assert!(!used);
    }
}
