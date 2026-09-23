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
