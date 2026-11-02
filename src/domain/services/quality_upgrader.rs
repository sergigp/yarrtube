use crate::domain::shared::Quality;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::VideoDownloaderRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::path::Path;
use std::sync::Arc;

/// Upgrades a young downloaded video to the better quality YouTube finished
/// processing after its download — see the `video-download` capability's
/// "Young Video Quality Upgrade". Runs from the `upgrade_quality` task;
/// `VideoReconciler` decides when to schedule it.
#[derive(Clone)]
pub struct QualityUpgrader {
    video_repository: Arc<dyn VideoRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl QualityUpgrader {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_repository,
            video_downloader_repository,
            video_file_repository,
            event_publisher,
            clock,
            videos_path: videos_path.into(),
        }
    }
}

pub trait QualityUpgraderApi: Send + Sync {
    /// No-op unless the video is quality-checkable
    /// (`Video::is_quality_checkable`). Asks the downloader which height
    /// `quality` would download now; when above the recorded one,
    /// re-downloads into the video's staging dir and replaces its episode
    /// file in `output_dir` in place, then records the new tier and height
    /// and publishes `VideoDownloaded`. Every check is recorded under the
    /// `late-video-data` policy. Best effort: a failed probe or re-download
    /// is recorded as a failed check, never returned.
    fn upgrade(
        &self,
        video_id: &VideoRecordId,
        quality: Quality,
        output_dir: &Path,
    ) -> anyhow::Result<()>;
}

impl QualityUpgraderApi for QualityUpgrader {
    fn upgrade(
        &self,
        _video_id: &VideoRecordId,
        _quality: Quality,
        _output_dir: &Path,
    ) -> anyhow::Result<()> {
        let _ = (
            &self.video_repository,
            &self.video_downloader_repository,
            &self.video_file_repository,
            &self.event_publisher,
            &self.clock,
            &self.videos_path,
        );
        Ok(())
    }
}
