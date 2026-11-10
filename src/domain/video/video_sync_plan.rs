use super::{Video, VideoSyncStep};
use chrono::{DateTime, Utc};

/// The steps a video's stored state still needs at `now`: each step that
/// is Pending, or Retrying with its next attempt due (see the
/// `video-retries` capability) — a thumbnail fetch while it has none and is
/// not being downloaded (the download records its own), the download, a
/// subtitle, chapters or quality check while that late data is due (see the
/// `late-video-data` capability), a summary while `summaries_wanted` (see
/// `video-summaries`), and a categorization while `categorization_wanted`
/// (see `video-categorization`). The thumbnail comes first so its task gets
/// the lower id and runs before the download holds the video's key. A step
/// already queued or running is left to `TaskRepository`'s dedupe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSyncPlan {
    pub steps: Vec<VideoSyncStep>,
}

impl VideoSyncPlan {
    pub fn for_video(
        video: &Video,
        summaries_wanted: bool,
        categorization_wanted: bool,
        now: DateTime<Utc>,
    ) -> Self {
        let steps = [
            (VideoSyncStep::FetchThumbnail, video.is_thumbnail_due(now)),
            (VideoSyncStep::Download, video.is_download_due(now)),
            (
                VideoSyncStep::FetchSubtitles,
                video.is_subtitle_fetchable() && video.subtitles.is_due(now),
            ),
            (
                VideoSyncStep::FetchChapters,
                video.is_chapters_fetchable() && video.chapters.is_due(now),
            ),
            (
                VideoSyncStep::UpgradeQuality,
                video.is_quality_checkable() && video.downloaded_quality.is_due(now),
            ),
            (
                VideoSyncStep::Summarize,
                summaries_wanted && video.is_summarizable() && video.summary.is_due(now),
            ),
            (
                VideoSyncStep::Categorize,
                categorization_wanted && video.is_categorization_due(now),
            ),
        ]
        .into_iter()
        .filter(|(_, needed)| *needed)
        .map(|(step, _)| step)
        .collect();
        Self { steps }
    }
}
