use super::errors::{CreatePlaylistError, DeletePlaylistError};
use super::playlist::Playlist;
use super::playlist_name::PlaylistName;
use super::youtube_playlist_id::YoutubePlaylistId;
use crate::domain::event::DomainEvent;
use crate::domain::task::Task;
use crate::domain::video::{Video, YoutubeVideoId};
use crate::infrastructure::repositories::sqlite_event_repository::EventPublisher;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::system_clock::Clock;
use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubePlaylistItemsRepository;
use crate::infrastructure::repositories::youtube_playlist_repository::YoutubePlaylistRepository;
use std::collections::HashSet;
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
    event_publisher: Arc<dyn EventPublisher>,
    video_repository: Arc<dyn VideoRepository>,
    youtube_playlist_repository: Arc<dyn YoutubePlaylistItemsRepository>,
    task_repository: Arc<dyn TaskRepository>,
    sync_interval_seconds: i64,
}

impl PlaylistService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repository: Arc<dyn PlaylistRepository>,
        lookup: Arc<dyn YoutubePlaylistRepository>,
        clock: Arc<dyn Clock>,
        event_publisher: Arc<dyn EventPublisher>,
        video_repository: Arc<dyn VideoRepository>,
        youtube_playlist_repository: Arc<dyn YoutubePlaylistItemsRepository>,
        task_repository: Arc<dyn TaskRepository>,
        sync_interval_seconds: i64,
    ) -> Self {
        Self {
            repository,
            lookup,
            clock,
            event_publisher,
            video_repository,
            youtube_playlist_repository,
            task_repository,
            sync_interval_seconds,
        }
    }

    pub fn create_playlist(
        &self,
        id: YoutubePlaylistId,
        name: PlaylistName,
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
        let playlist = Playlist::create(id, name, now);
        let event = DomainEvent::PlaylistCreated {
            playlist_id: playlist.id.as_str().to_string(),
        };
        self.repository
            .insert_with_event(&playlist, &event, now)
            .map_err(CreatePlaylistError::Repository)?;
        println!("[playlist] created {} ({})", playlist.id, playlist.name);
        Ok(CreatePlaylistOutcome::Created(playlist))
    }

    pub fn delete_playlist(&self, id: YoutubePlaylistId) -> Result<(), DeletePlaylistError> {
        match self.repository.find(&id) {
            Ok(Some(_)) => {}
            Ok(None) => return Err(DeletePlaylistError::NotFound(id)),
            Err(e) => return Err(DeletePlaylistError::Repository(e)),
        }

        let event = DomainEvent::PlaylistDeleted {
            playlist_id: id.as_str().to_string(),
        };
        self.repository
            .delete_with_event(&id, &event, self.clock.now())
            .map_err(DeletePlaylistError::Repository)?;
        println!("[playlist] deleted {id}");
        Ok(())
    }

    pub fn list_playlists(&self) -> anyhow::Result<Vec<Playlist>> {
        self.repository.list()
    }

    pub fn sync_playlist(&self, id: YoutubePlaylistId) -> anyhow::Result<()> {
        if self.repository.find(&id)?.is_none() {
            println!("[sync] playlist {id} no longer exists, skipping sync");
            return Ok(());
        }

        println!("[sync] syncing playlist {id}");
        let current_videos = self.youtube_playlist_repository.list_current_videos(&id)?;
        let stored_videos = self.video_repository.list_for_playlist(&id)?;
        let stored_ids: HashSet<&str> = stored_videos
            .iter()
            .map(|v| v.youtube_video_id.as_str())
            .collect();

        let now = self.clock.now();
        let mut current_ids = Vec::with_capacity(current_videos.len());
        for video in &current_videos {
            let youtube_video_id = YoutubeVideoId::new(&video.youtube_video_id)?;
            let is_new = !stored_ids.contains(youtube_video_id.as_str());

            self.video_repository.upsert(&Video::create(
                id.clone(),
                youtube_video_id.clone(),
                video.title.clone(),
                now,
            ))?;

            if is_new {
                println!(
                    "[sync] added video {} ({}) to playlist {id}",
                    youtube_video_id, video.title
                );
                self.event_publisher.publish(&DomainEvent::VideoAdded {
                    playlist_id: id.as_str().to_string(),
                    youtube_video_id: youtube_video_id.as_str().to_string(),
                })?;
            }

            current_ids.push(youtube_video_id);
        }

        let current_id_strs: HashSet<&str> = current_ids.iter().map(|v| v.as_str()).collect();
        for stored in &stored_videos {
            if !current_id_strs.contains(stored.youtube_video_id.as_str()) {
                println!(
                    "[sync] removing video {} from playlist {id} (no longer on YouTube)",
                    stored.youtube_video_id
                );
            }
        }
        self.video_repository.delete_not_in(&id, &current_ids)?;

        let next_run_at = now + chrono::Duration::seconds(self.sync_interval_seconds);
        self.task_repository.schedule(
            &Task::SyncPlaylist {
                playlist_id: id.as_str().to_string(),
            },
            next_run_at,
        )?;
        println!("[sync] scheduled next sync of playlist {id} at {next_run_at}");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::video::VideoStatus;
    use crate::infrastructure::repositories::sqlite_task_repository::PersistedTask;
    use crate::infrastructure::repositories::system_clock::Clock;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::PlaylistVideo;
    use crate::infrastructure::repositories::youtube_playlist_repository::YoutubePlaylistRepository;
    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakePlaylistRepository {
        playlists: Mutex<Vec<Playlist>>,
        /// Events written transactionally alongside a playlist insert/delete
        /// — this is what production code actually records them into (see
        /// `SqlitePlaylistRepository::insert_with_event`), not the injected
        /// `EventPublisher`.
        transactional_events: Mutex<Vec<DomainEvent>>,
    }

    impl FakePlaylistRepository {
        fn insert(&self, playlist: &Playlist) -> anyhow::Result<()> {
            self.playlists.lock().unwrap().push(playlist.clone());
            Ok(())
        }

        fn delete(&self, id: &YoutubePlaylistId) -> anyhow::Result<()> {
            self.playlists.lock().unwrap().retain(|p| p.id != *id);
            Ok(())
        }
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

        fn insert_with_event(
            &self,
            playlist: &Playlist,
            event: &DomainEvent,
            _now: DateTime<Utc>,
        ) -> anyhow::Result<()> {
            self.transactional_events
                .lock()
                .unwrap()
                .push(event.clone());
            self.insert(playlist)
        }

        fn delete_with_event(
            &self,
            id: &YoutubePlaylistId,
            event: &DomainEvent,
            _now: DateTime<Utc>,
        ) -> anyhow::Result<()> {
            self.transactional_events
                .lock()
                .unwrap()
                .push(event.clone());
            self.delete(id)
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

    #[derive(Default)]
    struct FakeEventPublisher {
        published: Mutex<Vec<DomainEvent>>,
    }

    impl EventPublisher for FakeEventPublisher {
        fn publish(&self, event: &DomainEvent) -> anyhow::Result<()> {
            self.published.lock().unwrap().push(event.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeVideoRepository {
        videos: Mutex<Vec<Video>>,
    }

    impl VideoRepository for FakeVideoRepository {
        fn upsert(&self, video: &Video) -> anyhow::Result<()> {
            let mut videos = self.videos.lock().unwrap();
            if let Some(existing) = videos.iter_mut().find(|v| {
                v.playlist_id == video.playlist_id && v.youtube_video_id == video.youtube_video_id
            }) {
                existing.title = video.title.clone();
                existing.updated_at = video.updated_at;
            } else {
                videos.push(video.clone());
            }
            Ok(())
        }

        fn list_for_playlist(&self, playlist_id: &YoutubePlaylistId) -> anyhow::Result<Vec<Video>> {
            Ok(self
                .videos
                .lock()
                .unwrap()
                .iter()
                .filter(|v| v.playlist_id == *playlist_id)
                .cloned()
                .collect())
        }

        fn delete_not_in(
            &self,
            playlist_id: &YoutubePlaylistId,
            current_ids: &[YoutubeVideoId],
        ) -> anyhow::Result<()> {
            self.videos.lock().unwrap().retain(|v| {
                v.playlist_id != *playlist_id || current_ids.contains(&v.youtube_video_id)
            });
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeYoutubePlaylistItemsRepository {
        videos: Vec<PlaylistVideo>,
    }

    impl YoutubePlaylistItemsRepository for FakeYoutubePlaylistItemsRepository {
        fn list_current_videos(
            &self,
            _playlist_id: &YoutubePlaylistId,
        ) -> anyhow::Result<Vec<PlaylistVideo>> {
            Ok(self.videos.clone())
        }
    }

    #[derive(Default)]
    struct FakeTaskRepository {
        scheduled: Mutex<Vec<(Task, DateTime<Utc>)>>,
    }

    impl TaskRepository for FakeTaskRepository {
        fn schedule(&self, task: &Task, run_at: DateTime<Utc>) -> anyhow::Result<()> {
            self.scheduled.lock().unwrap().push((task.clone(), run_at));
            Ok(())
        }

        fn list_eligible(&self) -> anyhow::Result<Vec<PersistedTask>> {
            Ok(Vec::new())
        }

        fn mark_running(&self, _id: i64) -> anyhow::Result<()> {
            Ok(())
        }

        fn mark_done(&self, _id: i64) -> anyhow::Result<()> {
            Ok(())
        }

        fn mark_failed_or_retry(&self, _id: i64, _error: &str) -> anyhow::Result<()> {
            Ok(())
        }

        fn recover_running(&self) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    struct ServiceBuilder {
        youtube_exists: bool,
        current_videos: Vec<PlaylistVideo>,
    }

    impl ServiceBuilder {
        fn new() -> Self {
            Self {
                youtube_exists: true,
                current_videos: Vec::new(),
            }
        }

        #[allow(clippy::type_complexity)]
        fn build(
            self,
        ) -> (
            PlaylistService,
            Arc<FakePlaylistRepository>,
            Arc<FakeEventPublisher>,
            Arc<FakeVideoRepository>,
            Arc<FakeTaskRepository>,
        ) {
            let playlist_repository = Arc::new(FakePlaylistRepository::default());
            let event_publisher = Arc::new(FakeEventPublisher::default());
            let video_repository = Arc::new(FakeVideoRepository::default());
            let task_repository = Arc::new(FakeTaskRepository::default());
            let service = PlaylistService::new(
                playlist_repository.clone(),
                Arc::new(FakeYoutubePlaylistRepository {
                    exists: self.youtube_exists,
                }),
                Arc::new(FixedClock(fixed_timestamp())),
                event_publisher.clone(),
                video_repository.clone(),
                Arc::new(FakeYoutubePlaylistItemsRepository {
                    videos: self.current_videos,
                }),
                task_repository.clone(),
                3600,
            );
            (
                service,
                playlist_repository,
                event_publisher,
                video_repository,
                task_repository,
            )
        }
    }

    fn service(youtube_exists: bool) -> PlaylistService {
        ServiceBuilder {
            youtube_exists,
            current_videos: Vec::new(),
        }
        .build()
        .0
    }

    fn pid(id: &str) -> YoutubePlaylistId {
        YoutubePlaylistId::new(id).unwrap()
    }

    fn pname(name: &str) -> PlaylistName {
        PlaylistName::new(name).unwrap()
    }

    #[test]
    fn it_should_create_the_playlist_when_the_youtube_playlist_exists() {
        let service = service(true);

        let outcome = service
            .create_playlist(pid("PL1"), pname("My Playlist"))
            .unwrap();

        assert_eq!(
            outcome,
            CreatePlaylistOutcome::Created(Playlist::create(
                pid("PL1"),
                pname("My Playlist"),
                fixed_timestamp(),
            ))
        );
    }

    #[test]
    fn it_should_return_the_existing_playlist_when_the_id_already_exists() {
        let service = service(true);
        service
            .create_playlist(pid("PL1"), pname("Original Name"))
            .unwrap();

        let outcome = service
            .create_playlist(pid("PL1"), pname("Different Name"))
            .unwrap();

        assert!(
            matches!(outcome, CreatePlaylistOutcome::AlreadyExisted(p) if p.name.as_str() == "Original Name")
        );
    }

    #[test]
    fn it_should_reject_creation_when_the_youtube_playlist_does_not_exist() {
        let service = service(false);

        let err = service
            .create_playlist(pid("PL404"), pname("My Playlist"))
            .unwrap_err();

        assert!(matches!(
            err,
            CreatePlaylistError::YoutubePlaylistNotFound(_)
        ));
    }

    #[test]
    fn it_should_publish_playlist_created_only_when_genuinely_created() {
        let (service, playlist_repository, _events, _videos, _tasks) =
            ServiceBuilder::new().build();

        service.create_playlist(pid("PL1"), pname("First")).unwrap();
        service
            .create_playlist(pid("PL1"), pname("First Again"))
            .unwrap();

        let published = playlist_repository.transactional_events.lock().unwrap();
        assert_eq!(
            *published,
            vec![DomainEvent::PlaylistCreated {
                playlist_id: "PL1".to_string()
            }]
        );
    }

    #[test]
    fn it_should_delete_an_existing_playlist() {
        let service = service(true);
        service
            .create_playlist(pid("PL1"), pname("My Playlist"))
            .unwrap();

        assert!(service.delete_playlist(pid("PL1")).is_ok());
    }

    #[test]
    fn it_should_publish_playlist_deleted_on_successful_deletion() {
        let (service, playlist_repository, _events, _videos, _tasks) =
            ServiceBuilder::new().build();
        service
            .create_playlist(pid("PL1"), pname("My Playlist"))
            .unwrap();

        service.delete_playlist(pid("PL1")).unwrap();

        let published = playlist_repository.transactional_events.lock().unwrap();
        assert_eq!(
            *published,
            vec![
                DomainEvent::PlaylistCreated {
                    playlist_id: "PL1".to_string()
                },
                DomainEvent::PlaylistDeleted {
                    playlist_id: "PL1".to_string()
                },
            ]
        );
    }

    #[test]
    fn it_should_report_not_found_when_deleting_a_missing_playlist() {
        let service = service(true);

        let err = service.delete_playlist(pid("PL404")).unwrap_err();

        assert!(matches!(err, DeletePlaylistError::NotFound(_)));
    }

    #[test]
    fn it_should_return_an_empty_list_when_no_playlists_exist() {
        let service = service(true);

        assert_eq!(service.list_playlists().unwrap(), Vec::new());
    }

    #[test]
    fn it_should_return_all_created_playlists() {
        let service = service(true);
        service.create_playlist(pid("PL1"), pname("First")).unwrap();
        service.create_playlist(pid("PL2"), pname("Second")).unwrap();

        assert_eq!(service.list_playlists().unwrap().len(), 2);
    }

    #[test]
    fn it_should_no_op_when_syncing_a_playlist_that_no_longer_exists() {
        let (service, _playlists, event_publisher, videos, tasks) = ServiceBuilder::new().build();

        service.sync_playlist(pid("PL404")).unwrap();

        assert!(event_publisher.published.lock().unwrap().is_empty());
        assert!(videos.videos.lock().unwrap().is_empty());
        assert!(tasks.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_persist_new_videos_and_publish_an_event_per_video() {
        let (service, _playlists, event_publisher, videos, _tasks) = ServiceBuilder {
            youtube_exists: true,
            current_videos: vec![PlaylistVideo {
                youtube_video_id: "vid1".to_string(),
                title: "One".to_string(),
            }],
        }
        .build();
        service
            .create_playlist(pid("PL1"), pname("My Playlist"))
            .unwrap();
        event_publisher.published.lock().unwrap().clear();

        service.sync_playlist(pid("PL1")).unwrap();

        let stored = videos.videos.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].status, VideoStatus::Pending);
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::VideoAdded {
                playlist_id: "PL1".to_string(),
                youtube_video_id: "vid1".to_string(),
            }]
        );
    }

    #[test]
    fn it_should_delete_and_log_videos_no_longer_present() {
        let (service, _playlists, _events, videos, _tasks) = ServiceBuilder {
            youtube_exists: true,
            current_videos: vec![],
        }
        .build();
        service
            .create_playlist(pid("PL1"), pname("My Playlist"))
            .unwrap();
        videos.videos.lock().unwrap().push(Video::create(
            pid("PL1"),
            YoutubeVideoId::new("vid1").unwrap(),
            "Stale",
            fixed_timestamp(),
        ));

        service.sync_playlist(pid("PL1")).unwrap();

        assert!(videos.videos.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_always_schedule_the_next_sync_even_with_no_changes() {
        let (service, _playlists, _events, _videos, tasks) = ServiceBuilder::new().build();
        service
            .create_playlist(pid("PL1"), pname("My Playlist"))
            .unwrap();

        service.sync_playlist(pid("PL1")).unwrap();

        let scheduled = tasks.scheduled.lock().unwrap();
        assert_eq!(scheduled.len(), 1);
        assert_eq!(
            scheduled[0].0,
            Task::SyncPlaylist {
                playlist_id: "PL1".to_string()
            }
        );
        assert_eq!(
            scheduled[0].1,
            fixed_timestamp() + chrono::Duration::seconds(3600)
        );
    }
}
