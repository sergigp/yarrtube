use super::{Video, VideoTarget};
use crate::domain::task::Task;

/// One step of a stored video's lifecycle, performed by its own task. Later
/// steps (e.g. category assignment, Plex labels) are added here, with their
/// rule in `VideoSyncPlan` and, for slow non-YouTube work, their own
/// `TaskLane`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoSyncStep {
    Download,
    FetchThumbnail,
    FetchSubtitles,
}

impl VideoSyncStep {
    /// The task performing this step for `video`, run in that task's own lane.
    pub fn task_for(&self, video: &Video, target: &VideoTarget) -> Task {
        let video_id = video.id.as_str().to_string();
        let output_dir = target.output_dir.to_string_lossy().to_string();
        match self {
            Self::Download => Task::DownloadVideo {
                video_id,
                quality: target.quality.as_str().to_string(),
                output_dir,
            },
            Self::FetchThumbnail => Task::FetchThumbnail {
                video_id,
                output_dir,
            },
            Self::FetchSubtitles => Task::FetchSubtitles {
                video_id,
                output_dir,
            },
        }
    }
}
