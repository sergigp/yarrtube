use super::errors::{CreatePlaylistError, DeletePlaylistError};
use super::playlist::Playlist;
use super::playlist_name::PlaylistName;
use super::quality::Quality;
use crate::domain::event::DomainEvent;
use crate::domain::shared::PlaylistId;
use crate::infrastructure::repositories::sqlite_event_repository::EventPublisher;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::system_clock::Clock;
use crate::infrastructure::repositories::youtube_playlist_repository::YoutubePlaylistRepository;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::sqlite_event_repository::FakeEventPublisher;
    use crate::infrastructure::repositories::sqlite_playlist_repository::FakePlaylistRepository;
    use crate::infrastructure::repositories::system_clock::FixedClock;
    use crate::infrastructure::repositories::youtube_playlist_repository::FakeYoutubePlaylistRepository;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn service_with(
        youtube_exists: bool,
    ) -> (
        PlaylistService,
        Arc<FakePlaylistRepository>,
        Arc<FakeEventPublisher>,
    ) {
        let repository = Arc::new(FakePlaylistRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let service = PlaylistService::new(
            repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository {
                exists: youtube_exists,
            }),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        (service, repository, event_publisher)
    }

    #[test]
    fn it_should_publish_playlist_created_when_a_new_playlist_is_created() {
        let (service, _repository, event_publisher) = service_with(true);

        service
            .create_playlist(
                PlaylistId::new("PL1").unwrap(),
                PlaylistName::new("My Playlist").unwrap(),
                Quality::High,
            )
            .unwrap();

        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::PlaylistCreated {
                playlist_id: "PL1".to_string()
            }]
        );
    }

    #[test]
    fn it_should_not_publish_an_event_when_the_playlist_already_existed() {
        let (service, _repository, event_publisher) = service_with(true);
        service
            .create_playlist(
                PlaylistId::new("PL1").unwrap(),
                PlaylistName::new("My Playlist").unwrap(),
                Quality::High,
            )
            .unwrap();
        event_publisher.published.lock().unwrap().clear();

        service
            .create_playlist(
                PlaylistId::new("PL1").unwrap(),
                PlaylistName::new("Different Name").unwrap(),
                Quality::High,
            )
            .unwrap();

        assert!(event_publisher.published.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_publish_playlist_deleted_when_an_existing_playlist_is_deleted() {
        let (service, _repository, event_publisher) = service_with(true);
        service
            .create_playlist(
                PlaylistId::new("PL1").unwrap(),
                PlaylistName::new("My Playlist").unwrap(),
                Quality::High,
            )
            .unwrap();
        event_publisher.published.lock().unwrap().clear();

        service
            .delete_playlist(PlaylistId::new("PL1").unwrap())
            .unwrap();

        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::PlaylistDeleted {
                playlist_id: "PL1".to_string()
            }]
        );
    }

    #[test]
    fn it_should_not_publish_an_event_when_deleting_a_missing_playlist() {
        let (service, _repository, event_publisher) = service_with(true);

        let result = service.delete_playlist(PlaylistId::new("PL404").unwrap());

        assert!(result.is_err());
        assert!(event_publisher.published.lock().unwrap().is_empty());
    }
}
