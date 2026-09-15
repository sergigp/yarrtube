use super::channel::Channel;
use super::channel_handle::ChannelHandle;
use super::errors::{CreateChannelError, DeleteChannelError};
use super::video_limit::VideoLimit;
use crate::domain::event::DomainEvent;
use crate::domain::shared::Quality;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::youtube_channel_repository::YoutubeChannelRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;
use tracing::info;

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
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
}

impl ChannelService {
    pub fn new(
        repository: Arc<dyn ChannelRepository>,
        lookup: Arc<dyn YoutubeChannelRepository>,
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

    pub fn create_channel(
        &self,
        id: ChannelHandle,
        quality: Quality,
        video_limit: VideoLimit,
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

        let now = self.clock.now();
        let channel = Channel::create(
            id,
            resolved.title,
            resolved.youtube_channel_id,
            quality,
            video_limit,
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

    pub fn delete_channel(&self, id: ChannelHandle) -> Result<(), DeleteChannelError> {
        match self.repository.find(&id) {
            Ok(Some(_)) => {}
            Ok(None) => return Err(DeleteChannelError::NotFound(id)),
            Err(e) => return Err(DeleteChannelError::Repository(e)),
        }

        self.repository
            .delete(&id)
            .map_err(DeleteChannelError::Repository)?;
        self.event_publisher
            .publish(&DomainEvent::ChannelDeleted {
                channel_id: id.as_str().to_string(),
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
    use crate::infrastructure::repositories::sqlite_channel_repository::FakeChannelRepository;
    use crate::infrastructure::repositories::youtube_channel_repository::{
        FakeYoutubeChannelRepository, ResolvedChannel,
    };
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn service(
        repository: FakeChannelRepository,
        resolved: Option<ResolvedChannel>,
    ) -> (ChannelService, Arc<FakeEventPublisher>) {
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let service = ChannelService::new(
            Arc::new(repository),
            Arc::new(FakeYoutubeChannelRepository { resolved }),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        (service, event_publisher)
    }

    fn resolved_channel() -> ResolvedChannel {
        ResolvedChannel {
            youtube_channel_id: "UC123".to_string(),
            title: "Some Channel".to_string(),
        }
    }

    #[test]
    fn it_should_create_a_new_channel_from_a_bare_handle() {
        let (service, event_publisher) =
            service(FakeChannelRepository::default(), Some(resolved_channel()));

        let outcome = service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::High,
                VideoLimit::new(10).unwrap(),
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
        let (service, event_publisher) =
            service(FakeChannelRepository::default(), Some(resolved_channel()));
        let created = service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::High,
                VideoLimit::new(10).unwrap(),
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
            )
            .unwrap();

        let CreateChannelOutcome::AlreadyExisted(channel) = outcome else {
            panic!("expected AlreadyExisted outcome");
        };
        assert_eq!(channel, original);
        assert_eq!(channel.quality, Quality::High);
        assert_eq!(channel.video_limit.value(), 10);

        let published = event_publisher.published.lock().unwrap();
        assert_eq!(published.len(), 1);
    }

    #[test]
    fn it_should_fail_when_the_youtube_channel_does_not_exist() {
        let (service, event_publisher) = service(FakeChannelRepository::default(), None);

        let result = service.create_channel(
            ChannelHandle::new("@missing").unwrap(),
            Quality::High,
            VideoLimit::new(10).unwrap(),
        );

        assert!(matches!(
            result,
            Err(CreateChannelError::YoutubeChannelNotFound(_))
        ));
        assert!(event_publisher.published.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_delete_an_existing_channel_and_publish_an_event() {
        let (service, event_publisher) =
            service(FakeChannelRepository::default(), Some(resolved_channel()));
        service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::High,
                VideoLimit::new(10).unwrap(),
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
                    channel_id: "@somechannel".to_string()
                },
            ]
        );
    }

    #[test]
    fn it_should_fail_and_not_publish_when_deleting_a_nonexistent_channel() {
        let (service, event_publisher) = service(FakeChannelRepository::default(), None);

        let result = service.delete_channel(ChannelHandle::new("@missing").unwrap());

        assert!(matches!(result, Err(DeleteChannelError::NotFound(_))));
        assert!(event_publisher.published.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_return_an_empty_list_when_no_channels_exist() {
        let (service, _event_publisher) = service(FakeChannelRepository::default(), None);

        assert_eq!(service.list_channels().unwrap(), Vec::new());
    }

    #[test]
    fn it_should_return_every_created_channel() {
        let (service, _event_publisher) =
            service(FakeChannelRepository::default(), Some(resolved_channel()));
        service
            .create_channel(
                ChannelHandle::new("@somechannel").unwrap(),
                Quality::High,
                VideoLimit::new(10).unwrap(),
            )
            .unwrap();

        let channels = service.list_channels().unwrap();

        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].id.as_str(), "@somechannel");
    }
}
