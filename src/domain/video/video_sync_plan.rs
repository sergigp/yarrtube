use super::{Video, VideoStatus, VideoSyncStep};
use chrono::{DateTime, Utc};

/// The steps a video's stored state still needs at `now`: a thumbnail
/// fetch while it has none and is neither being downloaded (the download
/// records its own) nor Excluded/Errored, a download until it settles, and a
/// subtitle, chapters or quality check while that late data is unsettled and
/// due (see the `late-video-data` capability), and a summary while
/// `summaries_wanted` and the video needs one (see `video-summaries`). The
/// thumbnail comes first so
/// its task gets the lower id and runs before the download holds the video's
/// key. A step already queued or running is left to `TaskRepository`'s
/// dedupe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSyncPlan {
    pub steps: Vec<VideoSyncStep>,
}

impl VideoSyncPlan {
    pub fn for_video(video: &Video, summaries_wanted: bool, now: DateTime<Utc>) -> Self {
        let steps = [
            (
                VideoSyncStep::FetchThumbnail,
                video.thumbnail_filename.is_none()
                    && video.is_thumbnail_fetchable()
                    && video.status != VideoStatus::InProgress,
            ),
            (VideoSyncStep::Download, !video.is_download_settled()),
            (
                VideoSyncStep::FetchSubtitles,
                video.is_subtitle_fetchable() && video.subtitle_checks.is_due(now),
            ),
            (
                VideoSyncStep::FetchChapters,
                video.is_chapters_fetchable() && video.chapters_checks.is_due(now),
            ),
            (
                VideoSyncStep::UpgradeQuality,
                video.is_quality_checkable() && video.quality_checks.is_due(now),
            ),
            (
                VideoSyncStep::Summarize,
                summaries_wanted && video.is_summarizable(),
            ),
        ]
        .into_iter()
        .filter(|(_, needed)| *needed)
        .map(|(step, _)| step)
        .collect();
        Self { steps }
    }
}
