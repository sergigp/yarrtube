use crate::domain::channel::{Channel, ChannelHandle, DeleteChannelError};
use crate::domain::event::DomainEvent;
use crate::infrastructure::repositories::filesystem_channel_avatar_repository::ChannelAvatarRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use std::sync::Arc;
use tracing::info;

/// Deletes a channel, its avatar, and every video row stored under it.
#[derive(Clone)]
pub struct ChannelDeleter {
    repository: Arc<dyn ChannelRepository>,
    video_repository: Arc<dyn VideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    avatar_repository: Arc<dyn ChannelAvatarRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl ChannelDeleter {
    pub fn new(
        repository: Arc<dyn ChannelRepository>,
        video_repository: Arc<dyn VideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        avatar_repository: Arc<dyn ChannelAvatarRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            repository,
            video_repository,
            channel_video_repository,
            avatar_repository,
            event_publisher,
        }
    }
}

pub trait ChannelDeleterApi: Send + Sync {
    /// Deletes a channel and every video row stored under it (its
    /// `ChannelVideo` rows and, for each, its owned `Video` row).
    fn delete(&self, id: ChannelHandle) -> Result<(), DeleteChannelError>;
}

impl ChannelDeleterApi for ChannelDeleter {
    fn delete(&self, id: ChannelHandle) -> Result<(), DeleteChannelError> {
        let channel = self.find_channel(&id)?;
        self.delete_channel_videos(&id)?;
        self.delete_avatar(&channel)?;
        self.delete_and_publish(&id, &channel)?;
        info!(channel_id = %id, "deleted channel");
        Ok(())
    }
}

impl ChannelDeleter {
    fn find_channel(&self, id: &ChannelHandle) -> Result<Channel, DeleteChannelError> {
        match self.repository.find(id) {
            Ok(Some(channel)) => Ok(channel),
            Ok(None) => Err(DeleteChannelError::NotFound(id.clone())),
            Err(e) => Err(DeleteChannelError::Repository(e)),
        }
    }

    /// Deletes each `ChannelVideo`'s owned `Video` row, then the
    /// `ChannelVideo` rows themselves.
    fn delete_channel_videos(&self, id: &ChannelHandle) -> Result<(), DeleteChannelError> {
        let channel_videos = self
            .channel_video_repository
            .list_for_channel(id)
            .map_err(DeleteChannelError::Repository)?;
        for channel_video in &channel_videos {
            self.video_repository
                .delete(&channel_video.video_id)
                .map_err(DeleteChannelError::Repository)?;
        }
        self.channel_video_repository
            .delete_all_for_channel(id)
            .map_err(DeleteChannelError::Repository)
    }

    fn delete_avatar(&self, channel: &Channel) -> Result<(), DeleteChannelError> {
        let Some(avatar_filename) = &channel.avatar_filename else {
            return Ok(());
        };
        self.avatar_repository
            .delete(avatar_filename)
            .map_err(DeleteChannelError::Repository)
    }

    fn delete_and_publish(
        &self,
        id: &ChannelHandle,
        channel: &Channel,
    ) -> Result<(), DeleteChannelError> {
        self.repository
            .delete(id)
            .map_err(DeleteChannelError::Repository)?;
        self.event_publisher
            .publish(&DomainEvent::ChannelDeleted {
                channel_id: id.as_str().to_string(),
                path: channel.path.as_str().to_string(),
            })
            .map_err(DeleteChannelError::Repository)
    }
}
