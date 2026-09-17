use super::channel::Channel;
use super::channel_handle::ChannelHandle;
use super::errors::{CreateChannelError, DeleteChannelError};
use super::video_limit::VideoLimit;
use crate::domain::event::DomainEvent;
use crate::domain::playlist::PlaylistPath;
use crate::domain::shared::Quality;
use crate::infrastructure::repositories::filesystem_channel_avatar_repository::ChannelAvatarRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_channel_repository::YoutubeChannelRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateChannelOutcome {
    Created(Channel),
    AlreadyExisted(Channel),
}

/// Orchestrates every operation on the channel aggregate. Injected with only the
/// ports channel operations actually use — not every port the application has.
#[derive(Clone)]
pub struct ChannelService {
    repository: Arc<dyn ChannelRepository>,
    lookup: Arc<dyn YoutubeChannelRepository>,
    avatar_repository: Arc<dyn ChannelAvatarRepository>,
    video_repository: Arc<dyn VideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
}

impl ChannelService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repository: Arc<dyn ChannelRepository>,
        lookup: Arc<dyn YoutubeChannelRepository>,
        avatar_repository: Arc<dyn ChannelAvatarRepository>,
        video_repository: Arc<dyn VideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repository,
            lookup,
            avatar_repository,
            video_repository,
            channel_video_repository,
            event_publisher,
            clock,
        }
    }

    pub fn create_channel(
        &self,
        id: ChannelHandle,
        quality: Quality,
        video_limit: VideoLimit,
        path: PlaylistPath,
    ) -> Result<CreateChannelOutcome, CreateChannelError> {
        match self.repository.find(&id) {
            Ok(Some(existing)) => return Ok(CreateChannelOutcome::AlreadyExisted(existing)),
            Ok(None) => {}
            Err(e) => return Err(CreateChannelError::Repository(e)),
        }

        let resolved = match self.lookup.resolve(&id) {
            Ok(Some(resolved)) => resolved,
            Ok(None) => return Err(CreateChannelError::YoutubeChannelNotFound(id)),
            Err(e) => return Err(CreateChannelError::Lookup(e)),
        };

        let avatar_filename =
            resolved
                .avatar_url
                .and_then(|url| match self.avatar_repository.store(&id, &url) {
                    Ok(filename) => filename,
                    Err(e) => {
                        warn!(channel_id = %id, error = %e, "failed to store channel avatar");
                        None
                    }
                });

        let now = self.clock.now();
        let channel = Channel::create(
            id,
            resolved.title,
            resolved.youtube_channel_id,
            quality,
            video_limit,
            path,
            avatar_filename,
            now,
        );
        self.repository
            .insert(&channel)
            .map_err(CreateChannelError::Repository)?;
        self.event_publisher
            .publish(&DomainEvent::ChannelCreated {
                channel_id: channel.id.as_str().to_string(),
            })
            .map_err(CreateChannelError::Repository)?;
        info!(channel_id = %channel.id, name = %channel.name, "created channel");
        Ok(CreateChannelOutcome::Created(channel))
    }

    /// Deletes a channel and every video row stored under it (its
    /// `ChannelVideo` rows and, for each, its owned `Video` row).
    pub fn delete_channel(&self, id: ChannelHandle) -> Result<(), DeleteChannelError> {
        let channel = match self.repository.find(&id) {
            Ok(Some(channel)) => channel,
            Ok(None) => return Err(DeleteChannelError::NotFound(id)),
            Err(e) => return Err(DeleteChannelError::Repository(e)),
        };

        let channel_videos = self
            .channel_video_repository
            .list_for_channel(&id)
            .map_err(DeleteChannelError::Repository)?;
        for channel_video in &channel_videos {
            self.video_repository
                .delete(&channel_video.video_id)
                .map_err(DeleteChannelError::Repository)?;
        }
        self.channel_video_repository
            .delete_all_for_channel(&id)
            .map_err(DeleteChannelError::Repository)?;

        if let Some(avatar_filename) = &channel.avatar_filename {
            self.avatar_repository
                .delete(avatar_filename)
                .map_err(DeleteChannelError::Repository)?;
        }

        self.repository
            .delete(&id)
            .map_err(DeleteChannelError::Repository)?;
        self.event_publisher
            .publish(&DomainEvent::ChannelDeleted {
                channel_id: id.as_str().to_string(),
                path: channel.path.as_str().to_string(),
            })
            .map_err(DeleteChannelError::Repository)?;
        info!(channel_id = %id, "deleted channel");
        Ok(())
    }

    pub fn list_channels(&self) -> anyhow::Result<Vec<Channel>> {
        self.repository.list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FakeChannelAvatarRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::FakeChannelRepository;
    use crate::infrastructure::repositories::sqlite_channel_video_repository::FakeChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::youtube_channel_repository::{
        FakeYoutubeChannelRepository, ResolvedChannel,
    };
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn path() -> PlaylistPath {
        PlaylistPath::new("creators/somechannel").unwrap()
    }

    fn service(
        repository: FakeChannelRepository,
        resolved: Option<ResolvedChannel>,
    ) -> (
        ChannelService,
        Arc<FakeEventPublisher>,
        Arc<FakeVideoRepository>,
        Arc<FakeChannelVideoRepository>,
    ) {
        let (service, event_publisher, video_repository, channel_video_repository, _avatars) =
            service_with_avatar_repository(
                repository,
                resolved,
                FakeChannelAvatarRepository::with_no_avatar(),
            );
        (
            service,
            event_publisher,
            video_repository,
            channel_video_repository,
        )
    }

    fn service_with_avatar_repository(
        repository: FakeChannelRepository,
        resolved: Option<ResolvedChannel>,
        avatar_repository: FakeChannelAvatarRepository,
    ) -> (
        ChannelService,
        Arc<FakeEventPublisher>,
        Arc<FakeVideoRepository>,
        Arc<FakeChannelVideoRepository>,
        Arc<FakeChannelAvatarRepository>,
    ) {
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let channel_video_repository = Arc::new(FakeChannelVideoRepository::default());
        let avatar_repository = Arc::new(avatar_repository);
        let service = ChannelService::new(
            Arc::new(repository),
            Arc::new(FakeYoutubeChannelRepository { resolved }),
            avatar_repository.clone(),
            video_repository.clone(),
            channel_video_repository.clone(),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        (
            service,
            event_publisher,
            video_repository,
            channel_video_repository,
            avatar_repository,
        )
    }

    fn resolved_channel() -> ResolvedChannel {
        ResolvedChannel {
            youtube_channel_id: "UC123".to_string(),
            title: "Some Channel".to_string(),
            avatar_url: None,
        }
    }

    fn resolved_channel_with_avatar() -> ResolvedChannel {
        ResolvedChannel {
            avatar_url: Some("https://example.com/avatar.jpg".to_string()),
            ..resolved_channel()
        }
    }

    #[test]
    fn it_should_create_a_new_channel_from_a_bare_handle() {
        let (service, event_publisher, _videos, _channel_videos) =
            service(FakeChannelRepository::default(), Some(resolved_channel()));

        let outcome = service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::High,
                VideoLimit::new(10).unwrap(),
                path(),
            )
            .unwrap();

        let CreateChannelOutcome::Created(channel) = outcome else {
            panic!("expected Created outcome");
        };
        assert_eq!(channel.id.as_str(), "@somechannel");
        assert_eq!(channel.name, "Some Channel");
        assert_eq!(channel.youtube_channel_id, "UC123");
        assert_eq!(channel.quality, Quality::High);
        assert_eq!(channel.video_limit.value(), 10);
        assert_eq!(channel.path, path());
        assert_eq!(channel.created_at, fixed_timestamp());

        let published = event_publisher.published.lock().unwrap();
        assert_eq!(
            *published,
            vec![DomainEvent::ChannelCreated {
                channel_id: "@somechannel".to_string()
            }]
        );
    }

    #[test]
    fn it_should_not_change_or_publish_when_the_channel_already_exists() {
        let (service, event_publisher, _videos, _channel_videos) =
            service(FakeChannelRepository::default(), Some(resolved_channel()));
        let created = service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::High,
                VideoLimit::new(10).unwrap(),
                path(),
            )
            .unwrap();
        let CreateChannelOutcome::Created(original) = created else {
            panic!("expected Created outcome");
        };

        let outcome = service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::Low,
                VideoLimit::new(5).unwrap(),
                PlaylistPath::new("different/path").unwrap(),
            )
            .unwrap();

        let CreateChannelOutcome::AlreadyExisted(channel) = outcome else {
            panic!("expected AlreadyExisted outcome");
        };
        assert_eq!(channel, original);
        assert_eq!(channel.quality, Quality::High);
        assert_eq!(channel.video_limit.value(), 10);
        assert_eq!(channel.path, path());

        let published = event_publisher.published.lock().unwrap();
        assert_eq!(published.len(), 1);
    }

    #[test]
    fn it_should_fail_when_the_youtube_channel_does_not_exist() {
        let (service, event_publisher, _videos, _channel_videos) =
            service(FakeChannelRepository::default(), None);

        let result = service.create_channel(
            ChannelHandle::new("@missing").unwrap(),
            Quality::High,
            VideoLimit::new(10).unwrap(),
            path(),
        );

        assert!(matches!(
            result,
            Err(CreateChannelError::YoutubeChannelNotFound(_))
        ));
        assert!(event_publisher.published.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_record_the_avatar_filename_when_the_avatar_repository_stores_one() {
        let (service, _events, _videos, _channel_videos, avatars) = service_with_avatar_repository(
            FakeChannelRepository::default(),
            Some(resolved_channel_with_avatar()),
            FakeChannelAvatarRepository::with_stored_filename("@somechannel.jpg"),
        );

        let outcome = service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::High,
                VideoLimit::new(10).unwrap(),
                path(),
            )
            .unwrap();

        let CreateChannelOutcome::Created(channel) = outcome else {
            panic!("expected Created outcome");
        };
        assert_eq!(
            channel.avatar_filename,
            Some("@somechannel.jpg".to_string())
        );
        assert_eq!(
            *avatars.stored_calls.lock().unwrap(),
            vec![(
                ChannelHandle::new("@somechannel").unwrap(),
                "https://example.com/avatar.jpg".to_string()
            )]
        );
    }

    #[test]
    fn it_should_not_record_an_avatar_filename_when_none_was_resolved() {
        let (service, _events, _videos, _channel_videos, _avatars) = service_with_avatar_repository(
            FakeChannelRepository::default(),
            Some(resolved_channel()),
            FakeChannelAvatarRepository::with_stored_filename("@somechannel.jpg"),
        );

        let outcome = service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::High,
                VideoLimit::new(10).unwrap(),
                path(),
            )
            .unwrap();

        let CreateChannelOutcome::Created(channel) = outcome else {
            panic!("expected Created outcome");
        };
        assert_eq!(channel.avatar_filename, None);
    }

    #[test]
    fn it_should_not_record_an_avatar_filename_when_the_avatar_repository_stores_none() {
        let (service, _events, _videos, _channel_videos, _avatars) = service_with_avatar_repository(
            FakeChannelRepository::default(),
            Some(resolved_channel_with_avatar()),
            FakeChannelAvatarRepository::with_no_avatar(),
        );

        let outcome = service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::High,
                VideoLimit::new(10).unwrap(),
                path(),
            )
            .unwrap();

        let CreateChannelOutcome::Created(channel) = outcome else {
            panic!("expected Created outcome");
        };
        assert_eq!(channel.avatar_filename, None);
    }

    #[test]
    fn it_should_delete_an_existing_channel_and_publish_an_event() {
        let (service, event_publisher, _videos, _channel_videos) =
            service(FakeChannelRepository::default(), Some(resolved_channel()));
        service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::High,
                VideoLimit::new(10).unwrap(),
                path(),
            )
            .unwrap();

        service
            .delete_channel(ChannelHandle::new("@somechannel").unwrap())
            .unwrap();

        assert_eq!(
            service
                .list_channels()
                .unwrap()
                .iter()
                .find(|c| c.id.as_str() == "@somechannel"),
            None
        );
        let published = event_publisher.published.lock().unwrap();
        assert_eq!(
            *published,
            vec![
                DomainEvent::ChannelCreated {
                    channel_id: "@somechannel".to_string()
                },
                DomainEvent::ChannelDeleted {
                    channel_id: "@somechannel".to_string(),
                    path: path().as_str().to_string(),
                },
            ]
        );
    }

    #[test]
    fn it_should_delete_the_avatar_file_when_deleting_a_channel_with_a_recorded_avatar() {
        let (service, _events, _videos, _channel_videos, avatars) = service_with_avatar_repository(
            FakeChannelRepository::default(),
            Some(resolved_channel_with_avatar()),
            FakeChannelAvatarRepository::with_stored_filename("@somechannel.jpg"),
        );
        service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::High,
                VideoLimit::new(10).unwrap(),
                path(),
            )
            .unwrap();

        service
            .delete_channel(ChannelHandle::new("@somechannel").unwrap())
            .unwrap();

        assert_eq!(
            *avatars.deleted_calls.lock().unwrap(),
            vec!["@somechannel.jpg".to_string()]
        );
    }

    #[test]
    fn it_should_not_attempt_avatar_deletion_when_no_avatar_was_recorded() {
        let (service, _events, _videos, _channel_videos, avatars) = service_with_avatar_repository(
            FakeChannelRepository::default(),
            Some(resolved_channel()),
            FakeChannelAvatarRepository::with_no_avatar(),
        );
        service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::High,
                VideoLimit::new(10).unwrap(),
                path(),
            )
            .unwrap();

        service
            .delete_channel(ChannelHandle::new("@somechannel").unwrap())
            .unwrap();

        assert!(avatars.deleted_calls.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_fail_and_not_publish_when_deleting_a_nonexistent_channel() {
        let (service, event_publisher, _videos, _channel_videos) =
            service(FakeChannelRepository::default(), None);

        let result = service.delete_channel(ChannelHandle::new("@missing").unwrap());

        assert!(matches!(result, Err(DeleteChannelError::NotFound(_))));
        assert!(event_publisher.published.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_return_an_empty_list_when_no_channels_exist() {
        let (service, _event_publisher, _videos, _channel_videos) =
            service(FakeChannelRepository::default(), None);

        assert_eq!(service.list_channels().unwrap(), Vec::new());
    }

    #[test]
    fn it_should_return_every_created_channel() {
        let (service, _event_publisher, _videos, _channel_videos) =
            service(FakeChannelRepository::default(), Some(resolved_channel()));
        service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::High,
                VideoLimit::new(10).unwrap(),
                path(),
            )
            .unwrap();

        let channels = service.list_channels().unwrap();

        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].id.as_str(), "@somechannel");
    }
}
