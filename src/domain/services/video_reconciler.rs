use crate::domain::category::CategoryMember;
use crate::domain::services::{
    MetadataGenerator, MetadataGeneratorApi, Relocation, VideoCategoryAssigner,
    VideoCategoryAssignerApi, VideoRelocator, VideoRelocatorApi,
};
use crate::domain::shared::Quality;
use crate::domain::video::{
    Video, VideoRecordId, VideoStatus, VideoSyncPlan, VideoTarget, nfo_path_for, resolve_output_dir,
};
use crate::domain::video_metadata::VideoMetadata;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use crate::infrastructure::shared::video_lock::VideoLock;
use std::path::Path;
use std::sync::Arc;
use tracing::warn;

const DOWNLOAD_VIDEO_TASK_TYPE: &str = "download_video";

/// The playlist or channel holding a video, as its sync needs it.
struct VideoOwner {
    source: CategoryMember,
    target: VideoTarget,
    /// The video's playlist position; `None` for a channel video, which
    /// sorts by publish date.
    sort_position: Option<i64>,
}

/// Owns a stored video's lifecycle — see the `video-sync` capability:
/// brings the video to its desired state and schedules each step still
/// needed as its own task.
#[derive(Clone)]
pub struct VideoReconciler {
    video_repository: Arc<dyn VideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    task_repository: Arc<dyn TaskRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    metadata_generator: Arc<MetadataGenerator>,
    video_relocator: Arc<VideoRelocator>,
    video_lock: Arc<VideoLock>,
    clock: Arc<dyn Clock>,
    videos_path: String,
    video_category_assigner: VideoCategoryAssigner,
}

impl VideoReconciler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        task_repository: Arc<dyn TaskRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        metadata_generator: Arc<MetadataGenerator>,
        video_relocator: Arc<VideoRelocator>,
        video_lock: Arc<VideoLock>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
        video_category_assigner: VideoCategoryAssigner,
    ) -> Self {
        Self {
            video_repository,
            channel_video_repository,
            playlist_video_repository,
            channel_repository,
            playlist_repository,
            task_repository,
            video_file_repository,
            video_metadata_repository,
            metadata_generator,
            video_relocator,
            video_lock,
            clock,
            videos_path: videos_path.into(),
            video_category_assigner,
        }
    }
}

pub trait VideoReconcilerApi: Send + Sync {
    /// Brings the stored video to its desired state, holding its lock:
    /// categories, broken-file reset, errored recovery, metadata repair,
    /// stranded warning, then schedules every step still needed. No-op if
    /// the video or its owner is gone. Idempotent.
    fn reconcile(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
}

impl VideoReconcilerApi for VideoReconciler {
    fn reconcile(&self, video_id: &VideoRecordId) -> anyhow::Result<()> {
        let _guard = self.video_lock.lock(video_id);
        let Some(video) = self.video_repository.find(video_id)? else {
            return Ok(());
        };
        let Some(owner) = self.find_owner(&video)? else {
            return Ok(());
        };
        self.video_category_assigner
            .assign_video(&video.id, &owner.source)?;
        let video = self.reset_if_broken(video, &owner.target)?;
        let video = self.recover_if_due(video)?;
        self.repair_metadata(&video, &owner);
        self.warn_if_stranded(&video)?;
        self.schedule_steps(&video, &owner.target)
    }
}

impl VideoReconciler {
    /// The channel holding `video`, else the playlist holding it; `None`
    /// when neither is found.
    fn find_owner(&self, video: &Video) -> anyhow::Result<Option<VideoOwner>> {
        match self.find_channel_owner(video)? {
            Some(owner) => Ok(Some(owner)),
            None => self.find_playlist_owner(video),
        }
    }

    fn find_channel_owner(&self, video: &Video) -> anyhow::Result<Option<VideoOwner>> {
        let Some(channel_video) = self.channel_video_repository.find_by_video(&video.id)? else {
            return Ok(None);
        };
        Ok(self
            .channel_repository
            .find(&channel_video.channel_id)?
            .map(|channel| VideoOwner {
                source: CategoryMember::Channel(channel.id),
                target: self.target(channel.path.as_str(), channel.quality),
                sort_position: None,
            }))
    }

    fn find_playlist_owner(&self, video: &Video) -> anyhow::Result<Option<VideoOwner>> {
        let Some(playlist_video) = self.playlist_video_repository.find_by_video(&video.id)? else {
            return Ok(None);
        };
        Ok(self
            .playlist_repository
            .find(&playlist_video.playlist_id)?
            .map(|playlist| VideoOwner {
                source: CategoryMember::Playlist(playlist.id),
                target: self.target(playlist.path.as_str(), playlist.quality),
                sort_position: Some(playlist_video.position),
            }))
    }

    fn target(&self, path: &str, quality: Quality) -> VideoTarget {
        VideoTarget {
            output_dir: resolve_output_dir(&self.videos_path, path),
            quality,
        }
    }

    /// Resets a `Downloaded` video whose file is missing or not mp4 (a stale
    /// container downloaded before yt-dlp always remuxed to mp4, see
    /// `args_for_quality`), so it is downloaded again; the pass's orphan
    /// sweep removes the stale file.
    fn reset_if_broken(&self, video: Video, target: &VideoTarget) -> anyhow::Result<Video> {
        if video.status != VideoStatus::Downloaded || self.has_healthy_file(&video, target) {
            return Ok(video);
        }
        warn!(
            video_id = %video.youtube_id,
            filename = video.filename.as_deref().unwrap_or(""),
            "downloaded video's file is missing or not mp4, resetting for redownload"
        );
        self.reset_for_redownload(video)
    }

    /// A video's file is healthy when it exists and is an mp4.
    fn has_healthy_file(&self, video: &Video, target: &VideoTarget) -> bool {
        video.filename.as_deref().is_some_and(|filename| {
            self.video_file_repository
                .file_exists(&target.output_dir, filename)
                && Path::new(filename)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
        })
    }

    fn reset_for_redownload(&self, video: Video) -> anyhow::Result<Video> {
        let reset = video.reset_for_redownload(self.clock.now());
        self.video_repository.update(&reset)?;
        Ok(reset)
    }

    /// Gives an `Errored` video (one that exhausted its download retries)
    /// another chance once its recovery cooldown is over
    /// (`Video::is_due_for_recovery`), with no limit on how many times a
    /// video may be recovered.
    fn recover_if_due(&self, video: Video) -> anyhow::Result<Video> {
        if !video.is_due_for_recovery(self.clock.now()) {
            return Ok(video);
        }
        warn!(
            video_id = %video.youtube_id,
            "permanently errored video is due for recovery, resetting for redownload"
        );
        self.reset_for_redownload(video)
    }

    /// Regenerates the metadata of a `Downloaded` video that has none, the
    /// same way `VideoDownloader` does at download time, referencing the
    /// thumbnail already on disk. Best effort: a failure is logged, the video
    /// is left as it is and its next sync tries again.
    fn repair_metadata(&self, video: &Video, owner: &VideoOwner) {
        if video.status != VideoStatus::Downloaded || self.has_metadata(video) {
            return;
        }
        let Some(metadata) = self
            .metadata_generator
            .generate(video, owner.sort_position, None)
        else {
            return;
        };
        let Some(video) = self.relocate_if_legacy(video, &metadata, &owner.target) else {
            return;
        };
        let (Some(filename), Some(number)) = (video.filename.as_deref(), video.episode_number)
        else {
            return;
        };
        let metadata = metadata.with_thumb(thumbnail_basename(&video));
        let nfo_path = owner.target.output_dir.join(nfo_path_for(filename));
        if let Err(e) = self
            .video_metadata_repository
            .save(&video.id, &metadata, &number, &nfo_path)
        {
            warn!(video_id = %video.id, error = %e, "failed to save video metadata during sync");
        }
    }

    /// `video` itself when already in its season folder, else `video` as
    /// stored once relocated there, now that its publish time is known;
    /// `None` when the relocation failed or moved nothing.
    fn relocate_if_legacy(
        &self,
        video: &Video,
        metadata: &VideoMetadata,
        target: &VideoTarget,
    ) -> Option<Video> {
        if !video.is_in_legacy_layout() {
            return Some(video.clone());
        }
        match self
            .video_relocator
            .relocate(video, Some(metadata.published_at), &target.output_dir)
        {
            Ok(Relocation::Moved { .. }) => self.video_repository.find(&video.id).ok().flatten(),
            Ok(_) => None,
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to relocate legacy video during sync");
                None
            }
        }
    }

    /// Whether `video` has metadata recorded; a failed read counts as
    /// having it, so nothing is regenerated over it.
    fn has_metadata(&self, video: &Video) -> bool {
        self.video_metadata_repository
            .find(&video.id)
            .inspect_err(|e| warn!(video_id = %video.id, error = %e, "failed to read video metadata during sync"))
            .map_or(true, |metadata| metadata.is_some())
    }

    /// Logs a video whose download started (In Progress or Errored Retrying)
    /// with no download pending or running: a step lost on a crash, which
    /// `schedule_steps` then schedules. A Pending video with no download is
    /// not stranded: a new or just reset video is scheduled by this sync.
    fn warn_if_stranded(&self, video: &Video) -> anyhow::Result<()> {
        if !matches!(
            video.status,
            VideoStatus::InProgress | VideoStatus::ErroredRetrying
        ) || self
            .task_repository
            .find_non_completed_for_video(DOWNLOAD_VIDEO_TASK_TYPE, &video.id)?
            .is_some()
        {
            return Ok(());
        }
        warn!(
            video_id = %video.youtube_id,
            status = video.status.as_str(),
            "stranded video found, scheduling its download"
        );
        Ok(())
    }

    fn schedule_steps(&self, video: &Video, target: &VideoTarget) -> anyhow::Result<()> {
        VideoSyncPlan::for_video(video)
            .steps
            .iter()
            .try_for_each(|step| {
                self.task_repository
                    .schedule(&step.task_for(video, target), self.clock.now())
            })
    }
}

/// The thumbnail's file name as an NFO references it, next to the episode.
fn thumbnail_basename(video: &Video) -> Option<String> {
    video
        .thumbnail_filename
        .as_deref()
        .and_then(|f| Path::new(f).file_name())
        .and_then(|f| f.to_str())
        .map(str::to_string)
}
