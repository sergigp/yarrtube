use crate::domain::channel::{Channel, ChannelHandle};
use crate::domain::channel_video::ChannelVideo;
use crate::domain::channel_video::{VideoAddedToChannel, VideoRemovedFromChannel};
use crate::domain::event::DomainEvent;
use crate::domain::services::{
    ChannelAvatarRestorer, ChannelAvatarRestorerApi, DesiredState, InternalVideoReconciler,
    InternalVideoReconcilerApi, MembershipDelta, ShowFileWriter, ShowFileWriterApi,
};
use crate::domain::task::Task;
use crate::domain::video::{Video, VideoId, VideoRecordId, VideoStatus};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::sqlite_video_transcript_repository::VideoTranscriptRepository;
use crate::infrastructure::repositories::youtube_channel_videos_repository::{
    ChannelVideoListing, ChannelVideosRepository,
};
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::error_report;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{info, info_span, warn};

/// Reconciles a channel with YouTube: brings its stored membership in line
/// with its current `video_limit` most recent uploads, then hands its folder
/// to `InternalVideoReconciler` to bring it in line with the database — the
/// channel equivalent of `PlaylistVideoReconciler`.
#[derive(Clone)]
pub struct ChannelVideoReconciler {
    channel_repository: Arc<dyn ChannelRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    channel_videos_repository: Arc<dyn ChannelVideosRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    task_repository: Arc<dyn TaskRepository>,
    internal_video_reconciler: Arc<InternalVideoReconciler>,
    show_file_writer: Arc<ShowFileWriter>,
    channel_avatar_restorer: Arc<ChannelAvatarRestorer>,
    clock: Arc<dyn Clock>,
}

impl ChannelVideoReconciler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        channel_repository: Arc<dyn ChannelRepository>,
        video_repository: Arc<dyn VideoRepository>,
        video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        channel_videos_repository: Arc<dyn ChannelVideosRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        task_repository: Arc<dyn TaskRepository>,
        internal_video_reconciler: Arc<InternalVideoReconciler>,
        show_file_writer: Arc<ShowFileWriter>,
        channel_avatar_restorer: Arc<ChannelAvatarRestorer>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            channel_repository,
            video_repository,
            video_transcript_repository,
            channel_video_repository,
            channel_videos_repository,
            event_publisher,
            task_repository,
            internal_video_reconciler,
            show_file_writer,
            channel_avatar_restorer,
            clock,
        }
    }
}

pub trait ChannelVideoReconcilerApi: Send + Sync {
    /// Runs one reconcile pass for a channel and reschedules the next
    /// recurring pass — no-ops entirely if the channel no longer exists.
    /// Called by the recurring `ReconcileChannelTask`. A failure to list
    /// the channel's videos is not fatal: membership is left untouched for
    /// this pass.
    fn reconcile(&self, id: ChannelHandle) -> anyhow::Result<()>;

    /// Runs the first reconcile pass of a newly created channel and
    /// schedules its recurring passes — no-ops entirely if the channel no
    /// longer exists. Called by the one-shot `ChannelCreated` reaction.
    /// Unlike `reconcile`, a failure to list the channel's videos fails the
    /// pass and schedules nothing, so the caller retries it instead of
    /// leaving the new channel empty until a recurring pass.
    fn initial_reconcile(&self, id: ChannelHandle) -> anyhow::Result<()>;

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

        self.reconcile_channel(channel, ListingFailure::SkipMembership)?;
        self.schedule_next_reconcile(&id)
    }

    fn initial_reconcile(&self, id: ChannelHandle) -> anyhow::Result<()> {
        let Some(channel) = self.find_channel(&id)? else {
            return Ok(());
        };

        self.reconcile_channel(channel, ListingFailure::FailPass)?;
        self.schedule_next_reconcile(&id)
    }

    fn force_reconcile(&self, id: ChannelHandle) -> anyhow::Result<()> {
        let Some(channel) = self.find_channel(&id)? else {
            return Ok(());
        };

        self.reconcile_channel(channel, ListingFailure::SkipMembership)
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

    /// One reconcile pass: restores the channel's avatar if its file is
    /// missing, brings the database in line with YouTube (the channel's
    /// membership), then the channel's folder in line with the database.
    /// Everything the pass logs, `InternalVideoReconciler`'s steps included,
    /// carries the channel id through the pass's span.
    fn reconcile_channel(
        &self,
        channel: Channel,
        on_listing_failure: ListingFailure,
    ) -> anyhow::Result<()> {
        let _span = info_span!("reconcile_channel", channel_id = %channel.id).entered();
        info!(channel_id = %channel.id, "reconciling channel");

        let channel = &self.channel_avatar_restorer.restore_missing(channel);
        // Best effort: a show file that can't be written must not stop the
        // source from syncing; the next pass tries again.
        if let Err(e) = self.show_file_writer.write_channel_show(channel) {
            warn!(channel_id = %channel.id, error = %e, "failed to write show files");
        }
        let delta = self.sync_membership_with_youtube(channel, on_listing_failure)?;
        let desired = self.desired_state(channel)?;
        self.internal_video_reconciler.reconcile(&desired, &delta)
    }

    /// Brings the stored membership in line with the channel's current
    /// `video_limit` most recent uploads: adds newly seen videos, refreshes
    /// the title and position of known ones, and removes stored videos no
    /// longer among them (removed on YouTube or aged past the limit). A
    /// failed listing either leaves membership untouched for this pass, the
    /// filesystem steps still running from the stored rows, or fails the
    /// pass, per `on_listing_failure`.
    fn sync_membership_with_youtube(
        &self,
        channel: &Channel,
        on_listing_failure: ListingFailure,
    ) -> anyhow::Result<MembershipDelta> {
        let Some(current_videos) = self.list_current_videos(channel, on_listing_failure)? else {
            return Ok(MembershipDelta::default());
        };
        let stored_videos = self.list_stored_videos(channel)?;
        let now = self.clock.now();

        // Keyed by YouTube id; a video added below joins it, so a video
        // listed twice is stored once.
        let mut known_videos: HashMap<String, (ChannelVideo, Video)> = stored_videos
            .iter()
            .map(|(cv, video)| {
                (
                    video.youtube_id.as_str().to_string(),
                    (cv.clone(), video.clone()),
                )
            })
            .collect();
        let mut delta = MembershipDelta::default();
        for current in &current_videos {
            match known_videos.get(&current.youtube_id) {
                None => {
                    let youtube_id = VideoId::new(&current.youtube_id)?;
                    let added = self.add_video(channel, youtube_id, current, now)?;
                    delta.added_ids.push(added.1.id.clone());
                    known_videos.insert(current.youtube_id.clone(), added);
                }
                Some((existing, video)) => {
                    self.refresh_known_video(existing, video, current, now)?
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

    /// The channel's current most recent uploads. When they can't be listed
    /// (see `channel-video-sync`'s "yt-dlp fails to list a channel's
    /// videos"), either `None` (logged) or the error, per
    /// `on_listing_failure`.
    fn list_current_videos(
        &self,
        channel: &Channel,
        on_listing_failure: ListingFailure,
    ) -> anyhow::Result<Option<Vec<ChannelVideoListing>>> {
        match (
            self.channel_videos_repository
                .list_current_videos(&channel.id, channel.video_limit.value()),
            on_listing_failure,
        ) {
            (Ok(videos), _) => Ok(Some(videos)),
            (Err(e), ListingFailure::FailPass) => Err(e.context("failed to list channel videos")),
            (Err(e), ListingFailure::SkipMembership) => {
                warn!(
                    channel_id = %channel.id,
                    error = %error_report::cause_chain(&e),
                    "failed to list channel videos"
                );
                Ok(None)
            }
        }
    }

    /// Every stored video of the channel with its membership, in recency
    /// order, read with one query per table.
    fn list_stored_videos(&self, channel: &Channel) -> anyhow::Result<Vec<(ChannelVideo, Video)>> {
        let channel_videos = self
            .channel_video_repository
            .list_for_channel(&channel.id)?;
        let video_ids: Vec<VideoRecordId> = channel_videos
            .iter()
            .map(|cv| cv.video_id.clone())
            .collect();
        let mut videos: HashMap<VideoRecordId, Video> = self
            .video_repository
            .find_many(&video_ids)?
            .into_iter()
            .map(|video| (video.id.clone(), video))
            .collect();

        Ok(channel_videos
            .into_iter()
            .filter_map(|cv| videos.remove(&cv.video_id).map(|video| (cv, video)))
            .collect())
    }

    /// Stores a newly seen video as `PENDING` at its recency position and
    /// announces it, which is what triggers its download and thumbnail.
    fn add_video(
        &self,
        channel: &Channel,
        youtube_id: VideoId,
        current: &ChannelVideoListing,
        now: DateTime<Utc>,
    ) -> anyhow::Result<(ChannelVideo, Video)> {
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

        Ok((channel_video, video))
    }

    /// Follows a known video's title and recency position on YouTube.
    fn refresh_known_video(
        &self,
        existing: &ChannelVideo,
        video: &Video,
        current: &ChannelVideoListing,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        if video.title != current.title {
            self.video_repository
                .update_title(&video.id, &current.title, now)?;
        }
        if existing.position != current.position {
            self.channel_video_repository.save(&ChannelVideo {
                position: current.position,
                created_at: now,
                ..existing.clone()
            })?;
        }

        Ok(())
    }

    /// Deletes every stored video whose YouTube id is not in
    /// `current_youtube_ids` and announces its removal, which is what
    /// cleans up its files.
    fn remove_videos_not_in(
        &self,
        channel: &Channel,
        stored_videos: &[(ChannelVideo, Video)],
        current_youtube_ids: &HashSet<&str>,
    ) -> anyhow::Result<()> {
        stored_videos
            .iter()
            .map(|(_, video)| video)
            .filter(|video| !current_youtube_ids.contains(video.youtube_id.as_str()))
            .try_for_each(|video| {
                info!(
                    channel_id = %channel.id,
                    video_id = %video.youtube_id,
                    "removing video from channel (no longer among its most recent uploads)"
                );
                self.channel_video_repository
                    .delete(&channel.id, &video.youtube_id)?;
                self.video_repository.delete(&video.id)?;
                self.video_transcript_repository.delete(&video.id)?;
                self.event_publisher
                    .publish(&DomainEvent::VideoRemovedFromChannel(
                        VideoRemovedFromChannel {
                            channel_id: channel.id.as_str().to_string(),
                            video_id: video.id.as_str().to_string(),
                            title: video.title.clone(),
                            filename: video.filename.clone(),
                            thumbnail_filename: video.thumbnail_filename.clone(),
                            subtitle_filename: video.subtitle_filename.clone(),
                            was_downloaded: video.status == VideoStatus::Downloaded,
                        },
                    ))
            })
    }

    /// What the database says the channel's folder should contain. A
    /// channel's videos sort by publish date, so it has no sort positions.
    fn desired_state(&self, channel: &Channel) -> anyhow::Result<DesiredState> {
        Ok(DesiredState {
            path: channel.path.clone(),
            quality: channel.quality,
            video_ids: self
                .channel_video_repository
                .list_for_channel(&channel.id)?
                .into_iter()
                .map(|cv| cv.video_id)
                .collect(),
            sort_positions: HashMap::new(),
        })
    }

    /// Re-reads the channel so an interval changed, or a channel deleted,
    /// while the pass ran is honoured. A channel keeps at most one pending
    /// reconcile; the running pass is not pending, so it never blocks itself.
    fn schedule_next_reconcile(&self, id: &ChannelHandle) -> anyhow::Result<()> {
        let Some(channel) = self.channel_repository.find(id)? else {
            info!(channel_id = %id, "channel deleted during reconcile, not scheduling the next one");
            return Ok(());
        };
        let task = Task::ReconcileChannel {
            channel_id: id.as_str().to_string(),
        };
        if self.task_repository.find_pending(&task)?.is_some() {
            info!(channel_id = %id, "a reconcile of the channel is already pending, not scheduling another");
            return Ok(());
        }
        let next_run_at = self.clock.now() + channel.reconcile_interval.duration();
        self.task_repository.schedule(&task, next_run_at)?;
        info!(channel_id = %id, next_run_at = %next_run_at, "scheduled next reconcile of channel");

        Ok(())
    }
}

/// What a pass does when listing the channel's videos on YouTube fails.
#[derive(Clone, Copy)]
enum ListingFailure {
    /// Leave membership untouched and run the rest of the pass.
    SkipMembership,
    /// Fail the pass.
    FailPass,
}
