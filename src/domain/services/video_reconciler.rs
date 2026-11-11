use crate::domain::preferences::Preferences;
use crate::domain::services::{
    MetadataGenerator, MetadataGeneratorApi, Relocation, VideoCategorizer, VideoCategorizerApi,
    VideoCategoryAssigner, VideoCategoryAssignerApi, VideoOwnerFinder, VideoOwnerFinderApi,
    VideoRelocator, VideoRelocatorApi, VideoUpdater, VideoUpdaterApi,
};
use crate::domain::video::{
    StepStatus, Video, VideoId, VideoOwner, VideoStatus, VideoSyncPlan, VideoTarget, nfo_path_for,
};
use crate::domain::video_metadata::VideoMetadata;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::path::Path;
use std::sync::Arc;
use tracing::warn;

const DOWNLOAD_VIDEO_TASK_TYPE: &str = "download_video";

/// Owns a stored video's lifecycle — see the `video-sync` capability:
/// brings the video to its desired state and schedules each step still
/// needed as its own task.
#[derive(Clone)]
pub struct VideoReconciler {
    video_repository: Arc<dyn VideoRepository>,
    video_updater: VideoUpdater,
    video_owner_finder: Arc<VideoOwnerFinder>,
    task_repository: Arc<dyn TaskRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    metadata_generator: Arc<MetadataGenerator>,
    video_relocator: Arc<VideoRelocator>,
    clock: Arc<dyn Clock>,
    video_category_assigner: VideoCategoryAssigner,
    preferences_repository: Arc<dyn PreferencesRepository>,
    video_categorizer: Arc<VideoCategorizer>,
}

impl VideoReconciler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        task_repository: Arc<dyn TaskRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        metadata_generator: Arc<MetadataGenerator>,
        video_relocator: Arc<VideoRelocator>,
        clock: Arc<dyn Clock>,
        video_category_assigner: VideoCategoryAssigner,
        preferences_repository: Arc<dyn PreferencesRepository>,
        video_categorizer: Arc<VideoCategorizer>,
    ) -> Self {
        Self {
            video_updater: VideoUpdater::new(video_repository.clone()),
            video_repository,
            video_owner_finder,
            task_repository,
            video_file_repository,
            video_metadata_repository,
            metadata_generator,
            video_relocator,
            clock,
            video_category_assigner,
            preferences_repository,
            video_categorizer,
        }
    }
}

pub trait VideoReconcilerApi: Send + Sync {
    /// Brings the stored video to its desired state (run by the `sync_video`
    /// task, so never alongside another task for the same video):
    /// categories, broken-file reset, metadata repair,
    /// stranded warning, then schedules every step still needed. No-op if
    /// the video or its owner is gone. Idempotent.
    fn reconcile(&self, video_id: &VideoId) -> anyhow::Result<()>;
}

impl VideoReconcilerApi for VideoReconciler {
    fn reconcile(&self, video_id: &VideoId) -> anyhow::Result<()> {
        let Some(video) = self.video_repository.find(video_id)? else {
            return Ok(());
        };
        let Some(owner) = self.video_owner_finder.find(&video)? else {
            return Ok(());
        };
        self.video_category_assigner
            .assign_video(&video.id, &owner.source)?;
        let video = self.reset_if_broken(video, &owner.target)?;
        self.repair_metadata(&video, &owner);
        self.warn_if_stranded(&video)?;
        let summaries_wanted = video.is_summarizable() && self.summaries_wanted(&owner)?;
        let categorization_wanted = self.categorization_wanted(&video, &owner)?;
        self.schedule_steps(&video, summaries_wanted, categorization_wanted)
    }
}

impl VideoReconciler {
    /// Resets a `Downloaded` video whose file is missing or not mp4 (a stale
    /// container downloaded before yt-dlp always remuxed to mp4, see
    /// `args_for_quality`), so it is downloaded again; the pass's orphan
    /// sweep removes the stale file.
    fn reset_if_broken(&self, video: Video, target: &VideoTarget) -> anyhow::Result<Video> {
        if video.download.status() != VideoStatus::Downloaded
            || self.has_healthy_file(&video, target)
        {
            return Ok(video);
        }
        warn!(
            youtube_id = %video.youtube_id,
            filename = video.download.filename().unwrap_or(""),
            "downloaded video's file is missing or not mp4, resetting for redownload"
        );
        self.reset_for_redownload(video)
    }

    /// A video's file is healthy when it exists and is an mp4.
    fn has_healthy_file(&self, video: &Video, target: &VideoTarget) -> bool {
        video.download.filename().is_some_and(|filename| {
            self.video_file_repository
                .file_exists(&target.output_dir, filename)
                && Path::new(filename)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
        })
    }

    fn reset_for_redownload(&self, video: Video) -> anyhow::Result<Video> {
        let now = self.clock.now();
        let reset = self
            .video_updater
            .update(&video.id, |video| Some(video.reset_for_redownload(now)))?;
        Ok(reset.unwrap_or(video))
    }

    /// Regenerates the metadata of a `Downloaded` video that has none, the
    /// same way `VideoDownloader` does at download time, referencing the
    /// thumbnail already on disk. Best effort: a failure is logged, the video
    /// is left as it is and its next sync tries again.
    fn repair_metadata(&self, video: &Video, owner: &VideoOwner) {
        if video.download.status() != VideoStatus::Downloaded || self.has_metadata(video) {
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
        let (Some(filename), Some(number)) = (video.download.filename(), video.episode_number)
        else {
            return;
        };
        let metadata = metadata.with_thumb(video.thumbnail.basename());
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
        if !video.download.is_in_legacy_layout() {
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

    /// Logs a video whose download started (In Progress) with no download
    /// pending or running: a step lost on a crash, which
    /// `schedule_steps` then schedules. A Pending video with no download is
    /// not stranded: a new or just reset video is scheduled by this sync.
    fn warn_if_stranded(&self, video: &Video) -> anyhow::Result<()> {
        if video.download.status() != VideoStatus::InProgress
            || self
                .task_repository
                .find_non_completed_for_video(DOWNLOAD_VIDEO_TASK_TYPE, &video.id)?
                .is_some()
        {
            return Ok(());
        }
        warn!(
            youtube_id = %video.youtube_id,
            status = video.download.status().as_str(),
            "stranded video found, scheduling its download"
        );
        Ok(())
    }

    /// Whether the videos of `owner` get an AI summary.
    fn summaries_wanted(&self, owner: &VideoOwner) -> anyhow::Result<bool> {
        Ok(owner.summaries_enabled && self.preferences()?.summaries_available())
    }

    /// The saved preferences, or the defaults.
    fn preferences(&self) -> anyhow::Result<Preferences> {
        Ok(self.preferences_repository.find()?.unwrap_or_default())
    }

    /// Whether `video` has categories pending for the AI.
    fn categorization_wanted(&self, video: &Video, owner: &VideoOwner) -> anyhow::Result<bool> {
        Ok(owner.categorization_enabled
            && self.preferences()?.categorization_available()
            && video.summary.status == StepStatus::Done
            && !self.video_categorizer.pending(video, owner)?.is_empty())
    }

    fn schedule_steps(
        &self,
        video: &Video,
        summaries_wanted: bool,
        categorization_wanted: bool,
    ) -> anyhow::Result<()> {
        VideoSyncPlan::for_video(
            video,
            summaries_wanted,
            categorization_wanted,
            self.clock.now(),
        )
        .steps
        .iter()
        .try_for_each(|step| {
            self.task_repository
                .schedule(&step.task_for(video), self.clock.now())
        })
    }
}
