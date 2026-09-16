use crate::domain::channel::{Channel, ChannelHandle};
use crate::domain::channel_video::ChannelVideo;
use crate::domain::event::DomainEvent;
use crate::domain::shared::VideoId;
use crate::domain::task::Task;
use crate::domain::video::{Video, VideoStatus};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_channel_videos_repository::ChannelVideosRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

/// Reconciles a channel's stored videos against its current `video_limit`
/// most recent uploads on YouTube, and its output directory against
/// recorded downloads — the channel equivalent of `VideoReconciler`.
#[derive(Clone)]
pub struct ChannelVideoReconciler {
    channel_repository: Arc<dyn ChannelRepository>,
    video_repository: Arc<dyn VideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    channel_videos_repository: Arc<dyn ChannelVideosRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    task_repository: Arc<dyn TaskRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    clock: Arc<dyn Clock>,
    reconcile_interval_seconds: i64,
    videos_path: String,
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
        video_file_repository: Arc<dyn VideoFileRepository>,
        clock: Arc<dyn Clock>,
        reconcile_interval_seconds: i64,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            channel_repository,
            video_repository,
            channel_video_repository,
            channel_videos_repository,
            event_publisher,
            task_repository,
            video_file_repository,
            clock,
            reconcile_interval_seconds,
            videos_path: videos_path.into(),
        }
    }

    /// Runs one reconcile pass for a channel and reschedules the next
    /// recurring pass — no-ops entirely if the channel no longer exists.
    pub fn reconcile(&self, id: ChannelHandle) -> anyhow::Result<()> {
        let Some(channel) = self.channel_repository.find(&id)? else {
            info!(channel_id = %id, "channel no longer exists, skipping reconcile");
            return Ok(());
        };

        self.run_reconcile_pass(&channel)?;

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

    /// Runs one reconcile pass for a channel immediately, on demand, without
    /// touching the recurring reconcile schedule. No-ops entirely if the
    /// channel no longer exists.
    pub fn force_reconcile(&self, id: ChannelHandle) -> anyhow::Result<()> {
        let Some(channel) = self.channel_repository.find(&id)? else {
            info!(channel_id = %id, "channel no longer exists, skipping reconcile");
            return Ok(());
        };

        self.run_reconcile_pass(&channel)
    }

    /// Diffs membership against YouTube, then reconciles the filesystem
    /// against recorded downloads. Shared by `reconcile` and
    /// `force_reconcile`. A `yt-dlp` failure during the membership diff
    /// propagates before filesystem reconciliation runs, leaving every
    /// stored row (and every file on disk) untouched.
    fn run_reconcile_pass(&self, channel: &Channel) -> anyhow::Result<()> {
        info!(channel_id = %channel.id, "reconciling channel");

        self.sync_channel_membership(channel)?;
        self.reconcile_filesystem(channel)
    }

    /// Diffs the channel's current `video_limit` most recent uploads against
    /// its stored `ChannelVideo` rows: adds newly-seen videos as `PENDING`
    /// at their recency position, evicts stored videos no longer among the
    /// current top-N (whether removed on YouTube or aged past the limit).
    fn sync_channel_membership(&self, channel: &Channel) -> anyhow::Result<()> {
        let id = &channel.id;
        let current_videos = self
            .channel_videos_repository
            .list_current_videos(id, channel.video_limit.value())?;
        let stored_videos = self.channel_video_repository.list_for_channel(id)?;

        let now = self.clock.now();
        let mut current_youtube_ids = Vec::with_capacity(current_videos.len());
        for current in &current_videos {
            let youtube_id = VideoId::new(&current.youtube_id)?;
            let existing = self
                .channel_video_repository
                .find_by_youtube_video(id, &youtube_id)?;

            match existing {
                None => {
                    let video = Video::create(youtube_id.clone(), current.title.clone(), now);
                    self.video_repository.save(&video)?;
                    let channel_video =
                        ChannelVideo::create(id.clone(), video.id.clone(), current.position, now);
                    self.channel_video_repository.save(&channel_video)?;
                    info!(
                        channel_id = %id,
                        video_id = %youtube_id,
                        title = %current.title,
                        "added video to channel"
                    );
                    self.event_publisher
                        .publish(&DomainEvent::VideoAddedToChannel {
                            channel_id: id.as_str().to_string(),
                            video_id: video.id.as_str().to_string(),
                        })?;
                }
                Some(existing) => {
                    if let Some(video) = self.video_repository.find(&existing.video_id)? {
                        self.video_repository.update(&Video {
                            title: current.title.clone(),
                            updated_at: now,
                            ..video
                        })?;
                    }
                    if existing.position != current.position {
                        self.channel_video_repository.save(&ChannelVideo {
                            position: current.position,
                            created_at: now,
                            ..existing
                        })?;
                    }
                }
            }

            current_youtube_ids.push(youtube_id);
        }

        let current_id_strs: HashSet<&str> =
            current_youtube_ids.iter().map(|v| v.as_str()).collect();
        for stored in &stored_videos {
            let Some(video) = self.video_repository.find(&stored.video_id)? else {
                continue;
            };
            if current_id_strs.contains(video.youtube_id.as_str()) {
                continue;
            }

            info!(
                channel_id = %id,
                video_id = %video.youtube_id,
                "evicting video from channel (no longer among its most recent uploads)"
            );
            self.channel_video_repository
                .delete(id, &video.youtube_id)?;
            self.video_repository.delete(&video.id)?;
            self.event_publisher
                .publish(&DomainEvent::VideoRemovedFromChannel {
                    channel_id: id.as_str().to_string(),
                    video_id: video.id.as_str().to_string(),
                    title: video.title.clone(),
                    filename: video.filename.clone(),
                    thumbnail_filename: video.thumbnail_filename.clone(),
                    was_downloaded: video.status == VideoStatus::Downloaded,
                })?;
        }

        Ok(())
    }

    /// Reconciles `channel`'s output directory against its recorded
    /// downloads: heals a `Downloaded` video whose file is missing, or whose
    /// file is present but not mp4, by resetting it and scheduling a fresh
    /// download. Also resets any `Errored` video (one that permanently
    /// exhausted its download retries) the same way. Also deletes a file
    /// that doesn't belong to any currently-`Downloaded` video (an orphan).
    /// Mirrors `VideoReconciler::reconcile_filesystem`.
    fn reconcile_filesystem(&self, channel: &Channel) -> anyhow::Result<()> {
        let output_dir = Path::new(&self.videos_path).join(channel.path.as_str());
        let files = self.video_file_repository.list(&output_dir)?;
        let stored_channel_videos = self
            .channel_video_repository
            .list_for_channel(&channel.id)?;
        let stored_videos: Vec<Video> = stored_channel_videos
            .iter()
            .filter_map(|cv| self.video_repository.find(&cv.video_id).transpose())
            .collect::<anyhow::Result<Vec<Video>>>()?;
        let downloaded: Vec<&Video> = stored_videos
            .iter()
            .filter(|v| v.status == VideoStatus::Downloaded)
            .collect();
        let protected_filenames: HashSet<&str> = downloaded
            .iter()
            .flat_map(|v| [v.filename.as_deref(), v.thumbnail_filename.as_deref()])
            .flatten()
            .collect();

        for video in &downloaded {
            let healthy = video.filename.as_deref().is_some_and(|filename| {
                files.iter().any(|f| f == filename)
                    && Path::new(filename)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
            });
            if healthy {
                continue;
            }

            let now = self.clock.now();
            warn!(
                channel_id = %channel.id,
                video_id = %video.youtube_id,
                filename = video.filename.as_deref().unwrap_or(""),
                "downloaded video's file is missing or not mp4, resetting for redownload"
            );
            let reset = (*video).clone().reset_for_redownload(now);
            self.video_repository.update(&reset)?;
            self.task_repository.schedule(
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: channel.quality.as_str().to_string(),
                    output_dir: output_dir.to_string_lossy().to_string(),
                },
                now,
            )?;
        }

        for video in stored_videos
            .iter()
            .filter(|v| v.status == VideoStatus::Errored)
        {
            let now = self.clock.now();
            warn!(
                channel_id = %channel.id,
                video_id = %video.youtube_id,
                "permanently errored video found during reconcile, resetting for redownload"
            );
            let reset = video.clone().reset_for_redownload(now);
            self.video_repository.update(&reset)?;
            self.task_repository.schedule(
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: channel.quality.as_str().to_string(),
                    output_dir: output_dir.to_string_lossy().to_string(),
                },
                now,
            )?;
        }

        for file in &files {
            if protected_filenames.contains(file.as_str()) {
                continue;
            }
            if self.video_file_repository.delete(&output_dir, file)? {
                info!(channel_id = %channel.id, file, "deleted orphaned file during reconciliation");
            }
        }

        Ok(())
    }
}
