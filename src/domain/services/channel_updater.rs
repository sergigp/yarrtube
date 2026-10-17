use crate::domain::channel::{Channel, ChannelHandle, UpdateChannelError, VideoLimit};
use crate::domain::shared::{Quality, ReconcileInterval};
use crate::domain::task::Task;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;
use tracing::info;

/// Changes the settings of an existing channel.
#[derive(Clone)]
pub struct ChannelUpdater {
    repository: Arc<dyn ChannelRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl ChannelUpdater {
    pub fn new(
        repository: Arc<dyn ChannelRepository>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repository,
            task_repository,
            clock,
        }
    }
}

pub trait ChannelUpdaterApi: Send + Sync {
    /// Sets the channel's quality, video limit and/or reconcile interval,
    /// returning the channel as stored. `None` keeps the stored value; quality
    /// and video limit apply from the channel's next sync. A new interval
    /// brings the pending reconcile forward when it would now run later than
    /// `now + interval`.
    fn update_settings(
        &self,
        id: ChannelHandle,
        quality: Option<Quality>,
        video_limit: Option<VideoLimit>,
        reconcile_interval: Option<ReconcileInterval>,
    ) -> Result<Channel, UpdateChannelError>;
}

impl ChannelUpdaterApi for ChannelUpdater {
    fn update_settings(
        &self,
        id: ChannelHandle,
        quality: Option<Quality>,
        video_limit: Option<VideoLimit>,
        reconcile_interval: Option<ReconcileInterval>,
    ) -> Result<Channel, UpdateChannelError> {
        let channel = Self::apply_settings(
            self.find_channel(&id)?,
            quality,
            video_limit,
            reconcile_interval,
        );
        self.update_channel(&channel)?;
        if let Some(reconcile_interval) = reconcile_interval {
            self.bring_forward_next_reconcile(&id, reconcile_interval)?;
        }
        info!(
            channel_id = %id,
            quality = channel.quality.as_str(),
            video_limit = channel.video_limit.value(),
            reconcile_interval_minutes = channel.reconcile_interval.minutes(),
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
        reconcile_interval: Option<ReconcileInterval>,
    ) -> Channel {
        let channel = match quality {
            Some(quality) => channel.with_quality(quality),
            None => channel,
        };
        let channel = match video_limit {
            Some(video_limit) => channel.with_video_limit(video_limit),
            None => channel,
        };
        match reconcile_interval {
            Some(reconcile_interval) => channel.with_reconcile_interval(reconcile_interval),
            None => channel,
        }
    }

    fn update_channel(&self, channel: &Channel) -> Result<(), UpdateChannelError> {
        self.repository
            .update(channel)
            .map_err(UpdateChannelError::Repository)
    }

    /// A shorter interval applies right away: the pending reconcile moves to
    /// `now + interval` when it was due later.
    fn bring_forward_next_reconcile(
        &self,
        id: &ChannelHandle,
        reconcile_interval: ReconcileInterval,
    ) -> Result<(), UpdateChannelError> {
        let task = Task::ReconcileChannel {
            channel_id: id.as_str().to_string(),
        };
        let Some(pending) = self
            .task_repository
            .find_pending(&task)
            .map_err(UpdateChannelError::Repository)?
        else {
            return Ok(());
        };
        let now = self.clock.now();
        self.task_repository
            .update(&pending.bring_forward(now + reconcile_interval.duration(), now))
            .map_err(UpdateChannelError::Repository)
    }
}
