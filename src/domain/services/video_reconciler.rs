use crate::domain::category::CategoryMember;
use crate::domain::services::{
    MetadataGenerator, VideoCategoryAssigner, VideoCategoryAssignerApi, VideoRelocator,
};
use crate::domain::shared::Quality;
use crate::domain::video::{
    Video, VideoRecordId, VideoStatus, VideoSyncPlan, VideoTarget, resolve_output_dir,
};
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

    fn repair_metadata(&self, _video: &Video, _owner: &VideoOwner) {}

    fn warn_if_stranded(&self, _video: &Video) -> anyhow::Result<()> {
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
