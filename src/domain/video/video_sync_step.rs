use super::Video;
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
    FetchChapters,
    UpgradeQuality,
    Summarize,
    Categorize,
}

impl VideoSyncStep {
    /// The task performing this step for `video`, run in that task's own
    /// lane. It carries only the video's id: what it needs from the video's
    /// owner (quality, show folder) is resolved when it runs.
    pub fn task_for(&self, video: &Video) -> Task {
        let video_id = video.id.as_str().to_string();
        match self {
            Self::Download => Task::DownloadVideo { video_id },
            Self::FetchThumbnail => Task::FetchThumbnail { video_id },
            Self::FetchSubtitles => Task::FetchSubtitles { video_id },
            Self::FetchChapters => Task::FetchChapters { video_id },
            Self::Summarize => Task::SummarizeVideo { video_id },
            Self::Categorize => Task::CategorizeVideo { video_id },
            Self::UpgradeQuality => Task::UpgradeQuality { video_id },
        }
    }
}
