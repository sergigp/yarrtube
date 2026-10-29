use super::{Video, VideoSyncStep};

/// The steps a video's stored state still needs, decided from that state
/// alone. A step already queued or running is left to `TaskRepository`'s
/// dedupe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSyncPlan {
    pub steps: Vec<VideoSyncStep>,
}

impl VideoSyncPlan {
    pub fn for_video(video: &Video) -> Self {
        let _ = video;
        Self {
            steps: vec![VideoSyncStep::Download, VideoSyncStep::FetchThumbnail],
        }
    }
}
