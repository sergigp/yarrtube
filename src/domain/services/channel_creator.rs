use crate::domain::channel::{Channel, ChannelHandle, CreateChannelError, VideoLimit};
use crate::domain::event::DomainEvent;
use crate::domain::playlist::PlaylistPath;
use crate::domain::shared::Quality;
use crate::infrastructure::repositories::filesystem_channel_avatar_repository::ChannelAvatarRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::youtube_channel_repository::{
    ResolvedChannel, YoutubeChannelRepository,
};
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
        if let Some(existing) = self.find_existing(&id)? {
            return Ok(CreateChannelOutcome::AlreadyExisted(existing));
        }

        let resolved = self.resolve_on_youtube(&id)?;
        let avatar_filename = self.store_avatar(&id, resolved.avatar_url);
        let channel = Channel::create(
            id,
            resolved.title,
            resolved.youtube_channel_id,
            quality,
            video_limit,
            path,
            avatar_filename,
            self.clock.now(),
        );
        self.insert_and_publish(&channel)?;
        Ok(CreateChannelOutcome::Created(channel))
    }
}

impl ChannelCreator {
    fn find_existing(&self, id: &ChannelHandle) -> Result<Option<Channel>, CreateChannelError> {
        self.repository
            .find(id)
            .map_err(CreateChannelError::Repository)
    }

    fn resolve_on_youtube(
        &self,
        id: &ChannelHandle,
    ) -> Result<ResolvedChannel, CreateChannelError> {
        match self.lookup.resolve(id) {
            Ok(Some(resolved)) => Ok(resolved),
            Ok(None) => Err(CreateChannelError::YoutubeChannelNotFound(id.clone())),
            Err(e) => Err(CreateChannelError::Lookup(e)),
        }
    }

    /// Best-effort: a failure to store the avatar is logged and the channel
    /// is created without one.
    fn store_avatar(&self, id: &ChannelHandle, avatar_url: Option<String>) -> Option<String> {
        avatar_url.and_then(|url| match self.avatar_repository.store(id, &url) {
            Ok(filename) => filename,
            Err(e) => {
                warn!(channel_id = %id, error = %e, "failed to store channel avatar");
                None
            }
        })
    }

    fn insert_and_publish(&self, channel: &Channel) -> Result<(), CreateChannelError> {
        self.repository
            .insert(channel)
            .map_err(CreateChannelError::Repository)?;
        self.event_publisher
            .publish(&DomainEvent::ChannelCreated {
                channel_id: channel.id.as_str().to_string(),
            })
            .map_err(CreateChannelError::Repository)?;
        info!(channel_id = %channel.id, name = %channel.name, "created channel");
        Ok(())
    }
}
