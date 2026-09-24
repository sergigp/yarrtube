use crate::domain::channel::{ChannelHandle, DeleteChannelError};
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
}
