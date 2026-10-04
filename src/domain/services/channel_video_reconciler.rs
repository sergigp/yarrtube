use crate::domain::channel::{Channel, ChannelHandle};
use crate::domain::channel_video::ChannelVideo;
use crate::domain::channel_video::{VideoAddedToChannel, VideoRemovedFromChannel};
use crate::domain::event::DomainEvent;
use crate::domain::services::{ThumbnailFetcher, ThumbnailFetcherApi};
use crate::domain::task::{ScheduledTask, Task};
use crate::domain::video::{
    Video, VideoStatus, resolve_output_dir, top_level_entry, video_dir_for_filename,
};
use crate::domain::video::{VideoId, VideoRecordId};
use crate::domain::video_metadata::{build_video_metadata, resolve_sorttitle};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_channel_videos_repository::{
    ChannelVideoListing, ChannelVideosRepository,
};
use crate::infrastructure::repositories::youtube_metadata_repository::YoutubeMetadataRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::error_report;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

/// Reconciles a channel's stored videos against its current `video_limit`
/// most recent uploads on YouTube, and its output directory against
/// recorded downloads — the channel equivalent of `PlaylistVideoReconciler`.
#[derive(Clone)]
pub struct ChannelVideoReconciler {
    channel_repository: Arc<dyn ChannelRepository>,
    video_repository: Arc<dyn VideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    channel_videos_repository: Arc<dyn ChannelVideosRepository>,
    youtube_metadata_repository: Arc<dyn YoutubeMetadataRepository>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    task_repository: Arc<dyn TaskRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    thumbnail_fetcher: Arc<ThumbnailFetcher>,
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
        youtube_metadata_repository: Arc<dyn YoutubeMetadataRepository>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        task_repository: Arc<dyn TaskRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        thumbnail_fetcher: Arc<ThumbnailFetcher>,
        clock: Arc<dyn Clock>,
        reconcile_interval_seconds: i64,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            channel_repository,
            video_repository,
            channel_video_repository,
            channel_videos_repository,
            youtube_metadata_repository,
            video_metadata_repository,
            event_publisher,
            task_repository,
            video_file_repository,
            thumbnail_fetcher,
            clock,
            reconcile_interval_seconds,
            videos_path: videos_path.into(),
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

    /// One reconcile pass: brings the stored membership in line with YouTube,
    /// then brings the output directory in line with the stored videos.
    fn reconcile_channel(&self, channel: &Channel) -> anyhow::Result<()> {
        info!(channel_id = %channel.id, "reconciling channel");

        let membership = self.sync_membership_with_youtube(channel)?;
        let local = self.take_local_snapshot(channel, &membership)?;

        let redownloaded = self.redownload_broken_files(channel, &local)?;
        self.generate_missing_metadata(&local)?;
        let recovered = self.recover_errored_videos(channel, &local)?;

        // Added videos get their download and thumbnail from their own
        // video-added event, reset ones with their fresh download, so the
        // safety nets below skip them.
        let handled: HashSet<&VideoRecordId> = membership
            .added_ids
            .iter()
            .chain(redownloaded)
            .chain(recovered)
            .collect();
        self.reschedule_stranded_downloads(channel, &local, &handled)?;
        self.thumbnail_fetcher
            .schedule_missing(&local.videos, &handled, &local.output_dir)?;

        self.delete_orphaned_files(channel, &local)
    }

    /// Brings the stored membership in line with the channel's current
    /// `video_limit` most recent uploads: adds newly seen videos, refreshes
    /// the title and position of known ones, and removes stored videos no
    /// longer among them (removed on YouTube or aged past the limit). A
    /// failed listing is not fatal: membership is left untouched for this
    /// pass and the filesystem steps still run from the stored rows.
    fn sync_membership_with_youtube(&self, channel: &Channel) -> anyhow::Result<MembershipChanges> {
        let Some(current_videos) = self.list_current_videos(channel) else {
            return Ok(MembershipChanges::default());
        };
        let stored_videos = self
            .channel_video_repository
            .list_for_channel(&channel.id)?;
        let now = self.clock.now();

        let mut changes = MembershipChanges::default();
        for current in &current_videos {
            let youtube_id = VideoId::new(&current.youtube_id)?;
            match self
                .channel_video_repository
                .find_by_youtube_video(&channel.id, &youtube_id)?
            {
                None => {
                    let added_id = self.add_video(channel, youtube_id, current, now)?;
                    changes.added_ids.push(added_id);
                }
                Some(existing) => {
                    if let Some((video_id, previous_title)) =
                        self.refresh_known_video(existing, current, now)?
                    {
                        changes.previous_titles.insert(video_id, previous_title);
                    }
                }
            }
        }

        let current_youtube_ids: HashSet<&str> = current_videos
            .iter()
            .map(|current| current.youtube_id.as_str())
            .collect();
        self.remove_videos_not_in(channel, &stored_videos, &current_youtube_ids)?;

        Ok(changes)
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

    /// Snapshots the channel's stored videos and output directory once, so
    /// every filesystem step works from the same picture taken before any of
    /// them changes something.
    fn take_local_snapshot(
        &self,
        channel: &Channel,
        membership: &MembershipChanges,
    ) -> anyhow::Result<LocalSnapshot> {
        let downloads_in_flight = self.video_ids_with_download_in_flight()?;
        let output_dir = resolve_output_dir(&self.videos_path, channel.path.as_str());
        let files = self.video_file_repository.list(&output_dir)?;
        let videos = self.list_stored_videos(&channel.id)?;
        let broken_download_ids = videos
            .iter()
            .filter(|v| {
                v.status == VideoStatus::Downloaded && !self.has_healthy_file(v, &output_dir)
            })
            .map(|v| v.id.clone())
            .collect();
        let protected_entries = protected_entries(&videos, &membership.previous_titles);

        Ok(LocalSnapshot {
            output_dir,
            videos,
            files,
            downloads_in_flight,
            broken_download_ids,
            protected_entries,
        })
    }

    fn video_ids_with_download_in_flight(&self) -> anyhow::Result<HashSet<String>> {
        Ok(self
            .task_repository
            .list_non_completed()?
            .iter()
            .filter_map(ScheduledTask::download_video_id)
            .collect())
    }

    fn list_stored_videos(&self, id: &ChannelHandle) -> anyhow::Result<Vec<Video>> {
        self.channel_video_repository
            .list_for_channel(id)?
            .iter()
            .filter_map(|cv| self.video_repository.find(&cv.video_id).transpose())
            .collect()
    }

    /// A video's file is healthy when it exists and is an mp4.
    fn has_healthy_file(&self, video: &Video, output_dir: &Path) -> bool {
        video.filename.as_deref().is_some_and(|filename| {
            self.video_file_repository.file_exists(output_dir, filename)
                && Path::new(filename)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
        })
    }

    /// Resets every `Downloaded` video whose file is missing or not mp4 (a
    /// stale container downloaded before yt-dlp always remuxed to mp4, see
    /// `args_for_quality`) and schedules a fresh download for it; orphan
    /// cleanup removes the stale file once it is redownloaded under a fresh
    /// filename. Returns the ids it reset.
    fn redownload_broken_files<'a>(
        &self,
        channel: &Channel,
        local: &'a LocalSnapshot,
    ) -> anyhow::Result<Vec<&'a VideoRecordId>> {
        local
            .broken_downloads()
            .map(|video| {
                warn!(
                    channel_id = %channel.id,
                    video_id = %video.youtube_id,
                    filename = video.filename.as_deref().unwrap_or(""),
                    "downloaded video's file is missing or not mp4, resetting for redownload"
                );
                self.reset_for_redownload(channel, &local.output_dir, video)?;
                Ok(&video.id)
            })
            .collect()
    }

    /// Repairs the metadata of every healthy `Downloaded` video that has
    /// none.
    fn generate_missing_metadata(&self, local: &LocalSnapshot) -> anyhow::Result<()> {
        for video in local.healthy_downloads() {
            if self.video_metadata_repository.find(&video.id)?.is_none() {
                self.generate_metadata(video, &local.output_dir);
            }
        }

        Ok(())
    }

    /// Regenerates `video`'s metadata — the channel equivalent of
    /// `PlaylistVideoReconciler::generate_metadata`. A channel-tracked video's
    /// `sorttitle` always resolves via publish date: `ChannelVideo.position`
    /// is a recency rank, never passed in as a playlist position.
    fn generate_metadata(&self, video: &Video, output_dir: &Path) {
        let Some(filename) = video.filename.as_deref() else {
            return;
        };
        let metadata = match self.youtube_metadata_repository.find(&video.youtube_id) {
            Ok(Some(metadata)) => metadata,
            Ok(None) => {
                warn!(video_id = %video.id, "no YouTube metadata found for video, skipping metadata repair");
                return;
            }
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to fetch YouTube metadata during reconcile, skipping metadata repair");
                return;
            }
        };

        let thumbnail_filename = video
            .thumbnail_filename
            .as_deref()
            .and_then(|f| Path::new(f).file_name())
            .and_then(|f| f.to_str())
            .map(str::to_string);
        let sorttitle = resolve_sorttitle(&metadata.title, metadata.published_at, None);
        let video_metadata = build_video_metadata(
            &video.youtube_id,
            &metadata,
            sorttitle,
            thumbnail_filename,
            self.clock.now(),
        );
        let video_dir = video_dir_for_filename(output_dir, filename);

        if let Err(e) = self
            .video_metadata_repository
            .save(&video.id, &video_metadata, &video_dir)
        {
            warn!(video_id = %video.id, error = %e, "failed to save video metadata during reconcile");
        }
    }

    /// Gives every `Errored` video (one that exhausted its download retries)
    /// another chance once its recovery cooldown is over
    /// (`Video::is_due_for_recovery`), with no limit on how many times a
    /// video may be recovered. Returns the ids it reset.
    fn recover_errored_videos<'a>(
        &self,
        channel: &Channel,
        local: &'a LocalSnapshot,
    ) -> anyhow::Result<Vec<&'a VideoRecordId>> {
        let now = self.clock.now();
        local
            .videos
            .iter()
            .filter(|v| v.is_due_for_recovery(now))
            .map(|video| {
                warn!(
                    channel_id = %channel.id,
                    video_id = %video.youtube_id,
                    "permanently errored video found during reconcile, resetting for redownload"
                );
                self.reset_for_redownload(channel, &local.output_dir, video)?;
                Ok(&video.id)
            })
            .collect()
    }

    /// Safety net for a video still waiting to be downloaded that has no
    /// download task pending or running (e.g. lost on a crash): schedules
    /// one again. Skips the `handled` videos, already taken care of.
    fn reschedule_stranded_downloads(
        &self,
        channel: &Channel,
        local: &LocalSnapshot,
        handled: &HashSet<&VideoRecordId>,
    ) -> anyhow::Result<()> {
        local
            .videos
            .iter()
            .filter(|v| {
                !v.is_download_settled()
                    && !local.downloads_in_flight.contains(v.id.as_str())
                    && !handled.contains(&v.id)
            })
            .try_for_each(|video| {
                warn!(
                    channel_id = %channel.id,
                    video_id = %video.youtube_id,
                    status = video.status.as_str(),
                    "stranded video found during reconcile, rescheduling its download"
                );
                self.schedule_download(channel, &local.output_dir, video, self.clock.now())
            })
    }

    /// Deletes every top-level entry of the output directory that doesn't
    /// belong to a stored video.
    fn delete_orphaned_files(
        &self,
        channel: &Channel,
        local: &LocalSnapshot,
    ) -> anyhow::Result<()> {
        for file in &local.files {
            if local.protected_entries.contains(file) {
                continue;
            }
            if self.video_file_repository.delete(&local.output_dir, file)? {
                info!(channel_id = %channel.id, file, "deleted orphaned file during reconciliation");
            }
        }

        Ok(())
    }

    fn reset_for_redownload(
        &self,
        channel: &Channel,
        output_dir: &Path,
        video: &Video,
    ) -> anyhow::Result<()> {
        let now = self.clock.now();
        self.video_repository
            .update(&video.clone().reset_for_redownload(now))?;
        self.schedule_download(channel, output_dir, video, now)
    }

    fn schedule_download(
        &self,
        channel: &Channel,
        output_dir: &Path,
        video: &Video,
        run_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        self.task_repository.schedule(
            &Task::DownloadVideo {
                video_id: video.id.as_str().to_string(),
                quality: channel.quality.as_str().to_string(),
                output_dir: output_dir.to_string_lossy().to_string(),
            },
            run_at,
        )
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

/// What a membership sync changed that the filesystem pass of the same
/// reconcile must know about.
#[derive(Default)]
struct MembershipChanges {
    /// Videos added by the sync: their thumbnail fetch comes from their own
    /// video-added event, not from the missing-thumbnail recovery.
    added_ids: Vec<VideoRecordId>,
    /// The title each renamed video had before the sync: a download started
    /// before the rename is still writing into the folder named after it.
    previous_titles: HashMap<VideoRecordId, String>,
}

/// A channel's stored videos and output directory as one pass found them,
/// before any filesystem step changes them.
struct LocalSnapshot {
    output_dir: PathBuf,
    videos: Vec<Video>,
    files: Vec<String>,
    /// Ids of every video with a download task pending or running.
    downloads_in_flight: HashSet<String>,
    /// `Downloaded` videos whose file is missing or not mp4.
    broken_download_ids: HashSet<VideoRecordId>,
    /// Top-level entries of `output_dir` that orphan cleanup must keep.
    protected_entries: HashSet<String>,
}

impl LocalSnapshot {
    fn broken_downloads(&self) -> impl Iterator<Item = &Video> {
        self.videos
            .iter()
            .filter(|v| self.broken_download_ids.contains(&v.id))
    }

    fn healthy_downloads(&self) -> impl Iterator<Item = &Video> {
        self.videos.iter().filter(|v| {
            v.status == VideoStatus::Downloaded && !self.broken_download_ids.contains(&v.id)
        })
    }
}

/// Top-level entries of the output directory orphan cleanup must keep:
/// every `Downloaded` video's file, every stored video's thumbnail (a
/// `Pending`/`InProgress` video may already have one pre-fetched, see the
/// `video-thumbnails` capability), and the folder an in-flight download or
/// thumbnail fetch is writing into — not recorded yet, so protected by its
/// predicted name.
fn protected_entries(
    videos: &[Video],
    previous_titles: &HashMap<VideoRecordId, String>,
) -> HashSet<String> {
    let downloaded_files = videos
        .iter()
        .filter(|v| v.status == VideoStatus::Downloaded)
        .filter_map(|v| v.filename.clone());
    let thumbnails = videos.iter().filter_map(|v| v.thumbnail_filename.clone());
    let unrecorded_folders = videos.iter().flat_map(|video| {
        video.unrecorded_folder_candidates(previous_titles.get(&video.id).map(String::as_str))
    });

    downloaded_files
        .chain(thumbnails)
        .chain(unrecorded_folders)
        .map(|entry| top_level_entry(&entry).to_string())
        .collect()
}
