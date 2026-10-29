use super::{Video, VideoStatus, VideoSyncStep};

/// The steps a video's stored state still needs, decided from that state
/// alone: a thumbnail fetch while it has none and is neither being
/// downloaded (the download records its own) nor Excluded/Errored, a
/// download until it settles, a subtitle fetch while it is
/// subtitle-fetchable. The thumbnail comes first so its task gets the lower
/// id and runs before the download holds the video's key. A step
/// already queued or running is left to `TaskRepository`'s dedupe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSyncPlan {
    pub steps: Vec<VideoSyncStep>,
}

impl VideoSyncPlan {
    pub fn for_video(video: &Video) -> Self {
        let steps = [
            (
                VideoSyncStep::FetchThumbnail,
                video.thumbnail_filename.is_none()
                    && video.is_thumbnail_fetchable()
                    && video.status != VideoStatus::InProgress,
            ),
            (VideoSyncStep::Download, !video.is_download_settled()),
            (VideoSyncStep::FetchSubtitles, video.is_subtitle_fetchable()),
        ]
        .into_iter()
        .filter(|(_, needed)| *needed)
        .map(|(step, _)| step)
        .collect();
        Self { steps }
    }
}
