use crate::domain::channel::{Channel, ChannelHandle};
use crate::domain::channel_video::ChannelVideo;
use crate::domain::channel_video::{VideoAddedToChannel, VideoRemovedFromChannel};
use crate::domain::event::DomainEvent;
use crate::domain::services::{
    DesiredState, InternalVideoReconciler, InternalVideoReconcilerApi, MembershipDelta,
};
use crate::domain::task::Task;
use crate::domain::video::{Video, VideoId, VideoRecordId, VideoStatus};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_channel_videos_repository::{
    ChannelVideoListing, ChannelVideosRepository,
};
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::error_report;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{info, warn};

/// Reconciles a channel with YouTube: brings its stored membership in line
/// with its current `video_limit` most recent uploads, then hands its folder
/// to `InternalVideoReconciler` to bring it in line with the database — the
/// channel equivalent of `PlaylistVideoReconciler`.
#[derive(Clone)]
pub struct ChannelVideoReconciler {
    channel_repository: Arc<dyn ChannelRepository>,
    video_repository: Arc<dyn VideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    channel_videos_repository: Arc<dyn ChannelVideosRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    task_repository: Arc<dyn TaskRepository>,
    internal_video_reconciler: Arc<InternalVideoReconciler>,
    clock: Arc<dyn Clock>,
    reconcile_interval_seconds: i64,
}

impl ChannelVideoReconciler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        channel_repository: Arc<dyn ChannelRepository>,
        video_repository: Arc<dyn VideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        channel_videos_repository: Arc<dyn ChannelVideosRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        task_repository: Arc<dyn TaskRepository>,
        internal_video_reconciler: Arc<InternalVideoReconciler>,
        clock: Arc<dyn Clock>,
        reconcile_interval_seconds: i64,
    ) -> Self {
        Self {
            channel_repository,
            video_repository,
            channel_video_repository,
            channel_videos_repository,
            event_publisher,
            task_repository,
            internal_video_reconciler,
            clock,
            reconcile_interval_seconds,
        }
    }
}

pub trait ChannelVideoReconcilerApi: Send + Sync {
    /// Runs one reconcile pass for a channel and reschedules the next
    /// recurring pass — no-ops entirely if the channel no longer exists.
    fn reconcile(&self, id: ChannelHandle) -> anyhow::Result<()>;

    /// Runs one reconcile pass for a channel immediately, on demand, without
    /// touching the recurring reconcile schedule. No-ops entirely if the
    /// channel no longer exists.
    fn force_reconcile(&self, id: ChannelHandle) -> anyhow::Result<()>;
}

impl ChannelVideoReconcilerApi for ChannelVideoReconciler {
    fn reconcile(&self, id: ChannelHandle) -> anyhow::Result<()> {
        let Some(channel) = self.find_channel(&id)? else {
            return Ok(());
        };

        self.reconcile_channel(&channel)?;
        self.schedule_next_reconcile(&id)
    }

    fn force_reconcile(&self, id: ChannelHandle) -> anyhow::Result<()> {
        let Some(channel) = self.find_channel(&id)? else {
            return Ok(());
        };

        self.reconcile_channel(&channel)
    }
}

impl ChannelVideoReconciler {
    fn find_channel(&self, id: &ChannelHandle) -> anyhow::Result<Option<Channel>> {
        let channel = self.channel_repository.find(id)?;
        if channel.is_none() {
            info!(channel_id = %id, "channel no longer exists, skipping reconcile");
        }
        Ok(channel)
    }

    /// One reconcile pass: brings the database in line with YouTube (the
    /// channel's membership), then the channel's folder in line with the
    /// database.
    fn reconcile_channel(&self, channel: &Channel) -> anyhow::Result<()> {
        info!(channel_id = %channel.id, "reconciling channel");

        let delta = self.sync_membership_with_youtube(channel)?;
        let desired = self.desired_state(channel)?;
        self.internal_video_reconciler.reconcile(&desired, &delta)
    }

    /// Brings the stored membership in line with the channel's current
    /// `video_limit` most recent uploads: adds newly seen videos, refreshes
    /// the title and position of known ones, and removes stored videos no
    /// longer among them (removed on YouTube or aged past the limit). A
    /// failed listing is not fatal: membership is left untouched for this
    /// pass and the filesystem steps still run from the stored rows.
    fn sync_membership_with_youtube(&self, channel: &Channel) -> anyhow::Result<MembershipDelta> {
        let Some(current_videos) = self.list_current_videos(channel) else {
            return Ok(MembershipDelta::default());
        };
        let stored_videos = self
            .channel_video_repository
            .list_for_channel(&channel.id)?;
        let now = self.clock.now();

        let mut delta = MembershipDelta::default();
        for current in &current_videos {
            let youtube_id = VideoId::new(&current.youtube_id)?;
            match self
                .channel_video_repository
                .find_by_youtube_video(&channel.id, &youtube_id)?
            {
                None => {
                    let added_id = self.add_video(channel, youtube_id, current, now)?;
                    delta.added_ids.push(added_id);
                }
                Some(existing) => {
                    if let Some((video_id, previous_title)) =
                        self.refresh_known_video(existing, current, now)?
                    {
                        delta.previous_titles.insert(video_id, previous_title);
                    }
                }
            }
        }

        let current_youtube_ids: HashSet<&str> = current_videos
            .iter()
            .map(|current| current.youtube_id.as_str())
            .collect();
        self.remove_videos_not_in(channel, &stored_videos, &current_youtube_ids)?;

        Ok(delta)
    }

    /// The channel's current most recent uploads, or `None` (logged) when
    /// they can't be listed — see `channel-video-sync`'s "yt-dlp fails to
    /// list a channel's videos".
    fn list_current_videos(&self, channel: &Channel) -> Option<Vec<ChannelVideoListing>> {
        self.channel_videos_repository
            .list_current_videos(&channel.id, channel.video_limit.value())
            .inspect_err(|e| {
                warn!(
                    channel_id = %channel.id,
                    error = %error_report::cause_chain(e),
                    "failed to list channel videos"
                );
            })
            .ok()
    }

    /// Stores a newly seen video as `PENDING` at its recency position and
    /// announces it, which is what triggers its download and thumbnail.
    fn add_video(
        &self,
        channel: &Channel,
        youtube_id: VideoId,
        current: &ChannelVideoListing,
        now: DateTime<Utc>,
    ) -> anyhow::Result<VideoRecordId> {
        let video = Video::create(youtube_id, current.title.clone(), now);
        self.video_repository.save(&video)?;
        let channel_video =
            ChannelVideo::create(channel.id.clone(), video.id.clone(), current.position, now);
        self.channel_video_repository.save(&channel_video)?;
        info!(
            channel_id = %channel.id,
            video_id = %video.youtube_id,
            title = %current.title,
            "added video to channel"
        );
        self.event_publisher
            .publish(&DomainEvent::VideoAddedToChannel(VideoAddedToChannel {
                channel_id: channel.id.as_str().to_string(),
                video_id: video.id.as_str().to_string(),
            }))?;

        Ok(video.id)
    }

    /// Follows a known video's title and recency position on YouTube.
    /// Returns the video's previous title when it was renamed.
    fn refresh_known_video(
        &self,
        existing: ChannelVideo,
        current: &ChannelVideoListing,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<(VideoRecordId, String)>> {
        let mut renamed = None;
        if let Some(video) = self.video_repository.find(&existing.video_id)?
            && video.title != current.title
        {
            self.video_repository
                .update_title(&video.id, &current.title, now)?;
            renamed = Some((video.id, video.title));
        }
        if existing.position != current.position {
            self.channel_video_repository.save(&ChannelVideo {
                position: current.position,
                created_at: now,
                ..existing
            })?;
        }

        Ok(renamed)
    }

    /// Deletes every stored video whose YouTube id is not in
    /// `current_youtube_ids` and announces its removal, which is what
    /// cleans up its files.
    fn remove_videos_not_in(
        &self,
        channel: &Channel,
        stored_videos: &[ChannelVideo],
        current_youtube_ids: &HashSet<&str>,
    ) -> anyhow::Result<()> {
        for stored in stored_videos {
            let Some(video) = self.video_repository.find(&stored.video_id)? else {
                continue;
            };
            if current_youtube_ids.contains(video.youtube_id.as_str()) {
                continue;
            }

            info!(
                channel_id = %channel.id,
                video_id = %video.youtube_id,
                "removing video from channel (no longer among its most recent uploads)"
            );
            self.channel_video_repository
                .delete(&channel.id, &video.youtube_id)?;
            self.video_repository.delete(&video.id)?;
            self.event_publisher
                .publish(&DomainEvent::VideoRemovedFromChannel(
                    VideoRemovedFromChannel {
                        channel_id: channel.id.as_str().to_string(),
                        video_id: video.id.as_str().to_string(),
                        title: video.title.clone(),
                        filename: video.filename.clone(),
                        thumbnail_filename: video.thumbnail_filename.clone(),
                        was_downloaded: video.status == VideoStatus::Downloaded,
                    },
                ))?;
        }

        Ok(())
    }

    /// What the database says the channel's folder should contain. A
    /// channel's videos sort by publish date, so it has no sort positions.
    fn desired_state(&self, channel: &Channel) -> anyhow::Result<DesiredState> {
        Ok(DesiredState {
            path: channel.path.clone(),
            quality: channel.quality,
            videos: self.list_stored_videos(&channel.id)?,
            sort_positions: HashMap::new(),
        })
    }

    fn list_stored_videos(&self, id: &ChannelHandle) -> anyhow::Result<Vec<Video>> {
        self.channel_video_repository
            .list_for_channel(id)?
            .iter()
            .filter_map(|cv| self.video_repository.find(&cv.video_id).transpose())
            .collect()
    }

    fn schedule_next_reconcile(&self, id: &ChannelHandle) -> anyhow::Result<()> {
        let now = self.clock.now();
        let next_run_at = now + chrono::Duration::seconds(self.reconcile_interval_seconds);
        self.task_repository.schedule(
            &Task::ReconcileChannel {
                channel_id: id.as_str().to_string(),
            },
            next_run_at,
        )?;
        info!(channel_id = %id, next_run_at = %next_run_at, "scheduled next reconcile of channel");

        Ok(())
    }
}
