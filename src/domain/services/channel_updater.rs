use crate::domain::channel::{Channel, ChannelHandle, UpdateChannelError, VideoLimit};
use crate::domain::shared::Quality;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use std::sync::Arc;
use tracing::info;

/// Changes the settings of an existing channel.
#[derive(Clone)]
pub struct ChannelUpdater {
    repository: Arc<dyn ChannelRepository>,
}

impl ChannelUpdater {
    pub fn new(repository: Arc<dyn ChannelRepository>) -> Self {
        Self { repository }
    }
}

pub trait ChannelUpdaterApi: Send + Sync {
    /// Sets the channel's quality and/or video limit, returning the channel
    /// as stored. `None` keeps the stored value; they apply from the
    /// channel's next sync.
    fn update_settings(
        &self,
        id: ChannelHandle,
        quality: Option<Quality>,
        video_limit: Option<VideoLimit>,
    ) -> Result<Channel, UpdateChannelError>;
}

impl ChannelUpdaterApi for ChannelUpdater {
    fn update_settings(
        &self,
        id: ChannelHandle,
        quality: Option<Quality>,
        video_limit: Option<VideoLimit>,
    ) -> Result<Channel, UpdateChannelError> {
        let channel = Self::apply_settings(self.find_channel(&id)?, quality, video_limit);
        self.update_channel(&channel)?;
        info!(
            channel_id = %id,
            quality = channel.quality.as_str(),
            video_limit = channel.video_limit.value(),
            "updated channel settings"
        );
        Ok(channel)
    }
}

impl ChannelUpdater {
    fn find_channel(&self, id: &ChannelHandle) -> Result<Channel, UpdateChannelError> {
        match self.repository.find(id) {
            Ok(Some(channel)) => Ok(channel),
            Ok(None) => Err(UpdateChannelError::NotFound(id.clone())),
            Err(e) => Err(UpdateChannelError::Repository(e)),
        }
    }

    fn apply_settings(
        channel: Channel,
        quality: Option<Quality>,
        video_limit: Option<VideoLimit>,
    ) -> Channel {
        let channel = match quality {
            Some(quality) => channel.with_quality(quality),
            None => channel,
        };
        match video_limit {
            Some(video_limit) => channel.with_video_limit(video_limit),
            None => channel,
        }
    }

    fn update_channel(&self, channel: &Channel) -> Result<(), UpdateChannelError> {
        self.repository
            .update(channel)
            .map_err(UpdateChannelError::Repository)
    }
}
