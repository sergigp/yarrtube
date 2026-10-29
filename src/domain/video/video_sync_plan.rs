use super::{Video, VideoStatus, VideoSyncStep};

/// The steps a video's stored state still needs, decided from that state
/// alone. A step already queued or running is left to `TaskRepository`'s
/// dedupe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSyncPlan {
    pub steps: Vec<VideoSyncStep>,
}

impl VideoSyncPlan {
    pub fn for_video(video: &Video) -> Self {
        let steps = [
            (VideoSyncStep::Download, !video.is_download_settled()),
            (
                VideoSyncStep::FetchThumbnail,
                video.thumbnail_filename.is_none() && video.status != VideoStatus::InProgress,
            ),
            (VideoSyncStep::FetchSubtitles, video.is_subtitle_fetchable()),
        ]
        .into_iter()
        .filter(|(_, needed)| *needed)
        .map(|(step, _)| step)
        .collect();
        Self { steps }
    }
}
