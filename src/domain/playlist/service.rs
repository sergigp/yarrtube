use super::errors::{CreatePlaylistError, DeletePlaylistError};
use super::playlist::Playlist;
use super::playlist_name::PlaylistName;
use super::youtube_playlist_id::YoutubePlaylistId;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::system_clock::Clock;
use crate::infrastructure::repositories::youtube_playlist_repository::YoutubePlaylistRepository;
use std::sync::Arc;

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
    clock: Arc<dyn Clock>,
}

impl PlaylistService {
    pub fn new(
        repository: Arc<dyn PlaylistRepository>,
        lookup: Arc<dyn YoutubePlaylistRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repository,
            lookup,
            clock,
        }
    }

    pub fn create_playlist(
        &self,
        id: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<CreatePlaylistOutcome, CreatePlaylistError> {
        let id = YoutubePlaylistId::new(id).map_err(CreatePlaylistError::InvalidInput)?;
        let name = PlaylistName::new(name).map_err(CreatePlaylistError::InvalidInput)?;

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

        let playlist = Playlist::create(id, name, self.clock.now());
        self.repository
            .insert(&playlist)
            .map_err(CreatePlaylistError::Repository)?;
        Ok(CreatePlaylistOutcome::Created(playlist))
    }

    pub fn delete_playlist(&self, id: impl Into<String>) -> Result<(), DeletePlaylistError> {
        let id = YoutubePlaylistId::new(id).map_err(DeletePlaylistError::InvalidId)?;

        match self.repository.find(&id) {
            Ok(Some(_)) => {}
            Ok(None) => return Err(DeletePlaylistError::NotFound(id)),
            Err(e) => return Err(DeletePlaylistError::Repository(e)),
        }

        self.repository
            .delete(&id)
            .map_err(DeletePlaylistError::Repository)
    }

    pub fn list_playlists(&self) -> anyhow::Result<Vec<Playlist>> {
        self.repository.list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
    use crate::infrastructure::repositories::system_clock::Clock;
    use crate::infrastructure::repositories::youtube_playlist_repository::YoutubePlaylistRepository;
    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakePlaylistRepository {
        playlists: Mutex<Vec<Playlist>>,
    }

    impl PlaylistRepository for FakePlaylistRepository {
        fn find(&self, id: &YoutubePlaylistId) -> anyhow::Result<Option<Playlist>> {
            Ok(self
                .playlists
                .lock()
                .unwrap()
                .iter()
                .find(|p| p.id == *id)
                .cloned())
        }

        fn insert(&self, playlist: &Playlist) -> anyhow::Result<()> {
            self.playlists.lock().unwrap().push(playlist.clone());
            Ok(())
        }

        fn delete(&self, id: &YoutubePlaylistId) -> anyhow::Result<()> {
            self.playlists.lock().unwrap().retain(|p| p.id != *id);
            Ok(())
        }

        fn list(&self) -> anyhow::Result<Vec<Playlist>> {
            Ok(self.playlists.lock().unwrap().clone())
        }
    }

    struct FakeYoutubePlaylistRepository {
        exists: bool,
    }

    impl YoutubePlaylistRepository for FakeYoutubePlaylistRepository {
        fn exists(&self, _id: &YoutubePlaylistId) -> anyhow::Result<bool> {
            Ok(self.exists)
        }
    }

    struct FixedClock(DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn service(youtube_exists: bool) -> PlaylistService {
        PlaylistService::new(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeYoutubePlaylistRepository {
                exists: youtube_exists,
            }),
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    #[test]
    fn it_should_create_the_playlist_when_the_youtube_playlist_exists() {
        let service = service(true);

        let outcome = service.create_playlist("PL1", "My Playlist").unwrap();

        assert_eq!(
            outcome,
            CreatePlaylistOutcome::Created(Playlist::create(
                YoutubePlaylistId::new("PL1").unwrap(),
                PlaylistName::new("My Playlist").unwrap(),
                fixed_timestamp(),
            ))
        );
    }

    #[test]
    fn it_should_return_the_existing_playlist_when_the_id_already_exists() {
        let service = service(true);
        service.create_playlist("PL1", "Original Name").unwrap();

        let outcome = service.create_playlist("PL1", "Different Name").unwrap();

        assert!(
            matches!(outcome, CreatePlaylistOutcome::AlreadyExisted(p) if p.name.as_str() == "Original Name")
        );
    }

    #[test]
    fn it_should_reject_creation_when_the_name_is_invalid() {
        let service = service(true);

        let err = service.create_playlist("PL1", "").unwrap_err();

        assert!(matches!(err, CreatePlaylistError::InvalidInput(_)));
    }

    #[test]
    fn it_should_reject_creation_when_the_youtube_playlist_does_not_exist() {
        let service = service(false);

        let err = service.create_playlist("PL404", "My Playlist").unwrap_err();

        assert!(matches!(
            err,
            CreatePlaylistError::YoutubePlaylistNotFound(_)
        ));
    }

    #[test]
    fn it_should_delete_an_existing_playlist() {
        let service = service(true);
        service.create_playlist("PL1", "My Playlist").unwrap();

        assert!(service.delete_playlist("PL1").is_ok());
    }

    #[test]
    fn it_should_report_not_found_when_deleting_a_missing_playlist() {
        let service = service(true);

        let err = service.delete_playlist("PL404").unwrap_err();

        assert!(matches!(err, DeletePlaylistError::NotFound(_)));
    }

    #[test]
    fn it_should_reject_deletion_when_the_id_is_invalid() {
        let service = service(true);

        let err = service.delete_playlist("").unwrap_err();

        assert!(matches!(err, DeletePlaylistError::InvalidId(_)));
    }

    #[test]
    fn it_should_return_an_empty_list_when_no_playlists_exist() {
        let service = service(true);

        assert_eq!(service.list_playlists().unwrap(), Vec::new());
    }

    #[test]
    fn it_should_return_all_created_playlists() {
        let service = service(true);
        service.create_playlist("PL1", "First").unwrap();
        service.create_playlist("PL2", "Second").unwrap();

        assert_eq!(service.list_playlists().unwrap().len(), 2);
    }
}
