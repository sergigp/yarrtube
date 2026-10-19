use crate::domain::channel::ChannelDeleted;
use crate::domain::channel::{Channel, ChannelHandle, DeleteChannelError};
use crate::domain::event::DomainEvent;
use crate::domain::task::Task;
use crate::infrastructure::repositories::filesystem_channel_avatar_repository::ChannelAvatarRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::sqlite_video_transcript_repository::VideoTranscriptRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use std::sync::Arc;
use tracing::info;

/// Deletes a channel, its avatar, and every video row stored under it.
#[derive(Clone)]
pub struct ChannelDeleter {
    repository: Arc<dyn ChannelRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    avatar_repository: Arc<dyn ChannelAvatarRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    task_repository: Arc<dyn TaskRepository>,
}

impl ChannelDeleter {
    pub fn new(
        repository: Arc<dyn ChannelRepository>,
        video_repository: Arc<dyn VideoRepository>,
        video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        avatar_repository: Arc<dyn ChannelAvatarRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        task_repository: Arc<dyn TaskRepository>,
    ) -> Self {
        Self {
            repository,
            video_repository,
            video_transcript_repository,
            channel_video_repository,
            avatar_repository,
            event_publisher,
            task_repository,
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
        self.remove_pending_reconciles(&id)?;
        self.delete_channel(&id)?;
        self.publish_channel_deleted(&id, &channel)?;
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

    /// Deletes each `ChannelVideo`'s owned `Video` row and its transcript,
    /// then the `ChannelVideo` rows themselves.
    fn delete_channel_videos(&self, id: &ChannelHandle) -> Result<(), DeleteChannelError> {
        let channel_videos = self
            .channel_video_repository
            .list_for_channel(id)
            .map_err(DeleteChannelError::Repository)?;
        for channel_video in &channel_videos {
            self.video_repository
                .delete(&channel_video.video_id)
                .map_err(DeleteChannelError::Repository)?;
            self.video_transcript_repository
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

    /// Removes every pending reconcile of the channel, so none outlives it. A
    /// database from before the one-pending-reconcile rule can hold several.
    fn remove_pending_reconciles(&self, id: &ChannelHandle) -> Result<(), DeleteChannelError> {
        let task = Task::ReconcileChannel {
            channel_id: id.as_str().to_string(),
        };
        let Some(pending) = self
            .task_repository
            .find_pending(&task)
            .map_err(DeleteChannelError::Repository)?
        else {
            return Ok(());
        };
        self.task_repository
            .delete(pending.id)
            .map_err(DeleteChannelError::Repository)?;
        self.remove_pending_reconciles(id)
    }

    fn delete_channel(&self, id: &ChannelHandle) -> Result<(), DeleteChannelError> {
        self.repository
            .delete(id)
            .map_err(DeleteChannelError::Repository)
    }

    fn publish_channel_deleted(
        &self,
        id: &ChannelHandle,
        channel: &Channel,
    ) -> Result<(), DeleteChannelError> {
        self.event_publisher
            .publish(&DomainEvent::ChannelDeleted(ChannelDeleted {
                channel_id: id.as_str().to_string(),
                name: channel.name.clone(),
                path: channel.path.as_str().to_string(),
            }))
            .map_err(DeleteChannelError::Repository)
    }
}
