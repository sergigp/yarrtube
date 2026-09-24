use crate::domain::channel::{Channel, ChannelHandle, CreateChannelError, VideoLimit};
use crate::domain::event::DomainEvent;
use crate::domain::playlist::PlaylistPath;
use crate::domain::shared::Quality;
use crate::infrastructure::repositories::filesystem_channel_avatar_repository::ChannelAvatarRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
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

/// Creates channels backed by a YouTube channel, storing its avatar when one
/// is available.
#[derive(Clone)]
pub struct ChannelCreator {
    repository: Arc<dyn ChannelRepository>,
    lookup: Arc<dyn YoutubeChannelRepository>,
    avatar_repository: Arc<dyn ChannelAvatarRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
}

impl ChannelCreator {
    pub fn new(
        repository: Arc<dyn ChannelRepository>,
        lookup: Arc<dyn YoutubeChannelRepository>,
        avatar_repository: Arc<dyn ChannelAvatarRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repository,
            lookup,
            avatar_repository,
            event_publisher,
            clock,
        }
    }
}

pub trait ChannelCreatorApi: Send + Sync {
    fn create(
        &self,
        id: ChannelHandle,
        quality: Quality,
        video_limit: VideoLimit,
        path: PlaylistPath,
    ) -> Result<CreateChannelOutcome, CreateChannelError>;
}

impl ChannelCreatorApi for ChannelCreator {
    fn create(
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
}
