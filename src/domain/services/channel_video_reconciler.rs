use crate::domain::channel::{Channel, ChannelHandle};
use crate::domain::channel_video::ChannelVideo;
use crate::domain::event::DomainEvent;
use crate::domain::services::{ThumbnailFetcher, ThumbnailFetcherApi};
use crate::domain::task::Task;
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
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

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
        let Some(channel) = self.channel_repository.find(&id)? else {
            info!(channel_id = %id, "channel no longer exists, skipping reconcile");
            return Ok(());
        };

        self.run_reconcile_pass(&channel)?;
        self.schedule_next_reconcile(&id)
    }

    fn force_reconcile(&self, id: ChannelHandle) -> anyhow::Result<()> {
        let Some(channel) = self.channel_repository.find(&id)? else {
            info!(channel_id = %id, "channel no longer exists, skipping reconcile");
            return Ok(());
        };

        self.run_reconcile_pass(&channel)
    }
}

impl ChannelVideoReconciler {
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

    /// Diffs membership against YouTube, then reconciles the filesystem
    /// against recorded downloads. Shared by `reconcile` and
    /// `force_reconcile`. A failed channel listing is not fatal: the
    /// membership diff is skipped for this pass, leaving every stored row
    /// untouched, and filesystem reconciliation still runs from them.
    fn run_reconcile_pass(&self, channel: &Channel) -> anyhow::Result<()> {
        info!(channel_id = %channel.id, "reconciling channel");

        let changes = self.sync_channel_membership(channel)?;

        self.reconcile_filesystem(channel, &changes)
    }

    /// The channel's current most recent uploads, or `None` (logged) when
    /// they can't be listed, so the caller skips membership changes instead
    /// of failing the pass — see `channel-video-sync`'s "yt-dlp fails to
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

    /// Diffs the channel's current `video_limit` most recent uploads against
    /// its stored `ChannelVideo` rows: adds newly-seen videos as `PENDING`
    /// at their recency position, evicts stored videos no longer among the
    /// current top-N (whether removed on YouTube or aged past the limit).
    fn sync_channel_membership(&self, channel: &Channel) -> anyhow::Result<MembershipChanges> {
        let id = &channel.id;
        let Some(current_videos) = self.list_current_videos(channel) else {
            return Ok(MembershipChanges::default());
        };
        let stored_videos = self.channel_video_repository.list_for_channel(id)?;

        let now = self.clock.now();
        let mut current_youtube_ids = Vec::with_capacity(current_videos.len());
        let mut added_ids = Vec::new();
        let mut previous_titles = HashMap::new();
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
                    added_ids.push(video.id);
                }
                Some(existing) => {
                    if let Some(video) = self.video_repository.find(&existing.video_id)?
                        && video.title != current.title
                    {
                        self.video_repository
                            .update_title(&video.id, &current.title, now)?;
                        previous_titles.insert(video.id, video.title);
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

        Ok(MembershipChanges {
            added_ids,
            previous_titles,
        })
    }

    /// Reconciles `channel`'s output directory against its recorded
    /// downloads: heals a `Downloaded` video whose file is missing, or whose
    /// file is present but not mp4, by resetting it and scheduling a fresh
    /// download. Also resets any `Errored` video (one that permanently
    /// exhausted its download retries) the same way once it has been errored
    /// for the recovery cooldown (`Video::is_due_for_recovery`). Also deletes a file
    /// that doesn't belong to any currently-`Downloaded` video (an orphan).
    /// Mirrors `PlaylistVideoReconciler::reconcile_filesystem`.
    fn reconcile_filesystem(
        &self,
        channel: &Channel,
        changes: &MembershipChanges,
    ) -> anyhow::Result<()> {
        let output_dir = resolve_output_dir(&self.videos_path, channel.path.as_str());
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
        // The folder a download or thumbnail fetch still in flight is writing
        // into isn't recorded yet, so it is protected by its predicted name.
        let unrecorded_folders: Vec<String> = stored_videos
            .iter()
            .flat_map(|video| {
                video.unrecorded_folder_candidates(
                    changes.previous_titles.get(&video.id).map(String::as_str),
                )
            })
            .collect();
        // Every stored video's thumbnail folder is protected regardless of
        // status: a `Pending`/`InProgress` video may already have a
        // pre-fetched thumbnail on disk, ahead of its own download — see
        // the `video-thumbnails` capability.
        let protected_top_level: HashSet<&str> = downloaded
            .iter()
            .filter_map(|v| v.filename.as_deref())
            .chain(
                stored_videos
                    .iter()
                    .filter_map(|v| v.thumbnail_filename.as_deref()),
            )
            .chain(unrecorded_folders.iter().map(String::as_str))
            .map(top_level_entry)
            .collect();
        // Videos the missing-thumbnail recovery below must skip. A video
        // added by this same pass gets its fetch from its own video-added
        // event, so recovery only covers videos stored before the pass. A
        // video reset for redownload below gets its thumbnail with its own
        // fresh download (see design.md's Non-Goals).
        let mut skip_thumbnail_ids: HashSet<&VideoRecordId> = changes.added_ids.iter().collect();

        for video in &downloaded {
            let healthy = video.filename.as_deref().is_some_and(|filename| {
                self.video_file_repository
                    .file_exists(&output_dir, filename)
                    && Path::new(filename)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
            });
            if !healthy {
                let now = self.clock.now();
                warn!(
                    channel_id = %channel.id,
                    video_id = %video.youtube_id,
                    filename = video.filename.as_deref().unwrap_or(""),
                    "downloaded video's file is missing or not mp4, resetting for redownload"
                );
                let reset = (*video).clone().reset_for_redownload(now);
                self.video_repository.update(&reset)?;
                skip_thumbnail_ids.insert(&video.id);
                self.task_repository.schedule(
                    &Task::DownloadVideo {
                        video_id: video.id.as_str().to_string(),
                        quality: channel.quality.as_str().to_string(),
                        output_dir: output_dir.to_string_lossy().to_string(),
                    },
                    now,
                )?;
                continue;
            }

            if self.video_metadata_repository.find(&video.id)?.is_some() {
                continue;
            }
            self.generate_metadata(video, &output_dir);
        }

        for video in stored_videos
            .iter()
            .filter(|v| v.is_due_for_recovery(self.clock.now()))
        {
            let now = self.clock.now();
            warn!(
                channel_id = %channel.id,
                video_id = %video.youtube_id,
                "permanently errored video found during reconcile, resetting for redownload"
            );
            let reset = video.clone().reset_for_redownload(now);
            self.video_repository.update(&reset)?;
            skip_thumbnail_ids.insert(&video.id);
            self.task_repository.schedule(
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: channel.quality.as_str().to_string(),
                    output_dir: output_dir.to_string_lossy().to_string(),
                },
                now,
            )?;
        }

        self.thumbnail_fetcher.schedule_missing(
            &stored_videos,
            &skip_thumbnail_ids,
            &output_dir,
        )?;

        for file in &files {
            if protected_top_level.contains(file.as_str()) {
                continue;
            }
            if self.video_file_repository.delete(&output_dir, file)? {
                info!(channel_id = %channel.id, file, "deleted orphaned file during reconciliation");
            }
        }

        Ok(())
    }
}
