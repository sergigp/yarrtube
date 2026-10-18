use crate::domain::video::Video;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::sqlite_video_transcript_repository::VideoTranscriptRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::VideoDownloaderRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

/// Best-effort fetch of a downloaded video's original-language subtitles —
/// see the `video-subtitles` capability. The fetch itself runs from the
/// `fetch_subtitles` task and never returns an error to its caller; each
/// reconcile pass's missing-subtitles recovery only schedules it.
#[derive(Clone)]
#[allow(dead_code)]
pub struct SubtitleFetcher {
    video_repository: Arc<dyn VideoRepository>,
    video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl SubtitleFetcher {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_repository,
            video_transcript_repository,
            video_downloader_repository,
            video_file_repository,
            task_repository,
            clock,
            videos_path: videos_path.into(),
        }
    }
}

pub trait SubtitleFetcherApi: Send + Sync {
    /// No-op unless `video` is subtitle-fetchable
    /// (`Video::is_subtitle_fetchable`). Otherwise probes its caption tracks,
    /// downloads the original-language one as `<episode stem>.<lang>.srt` in
    /// `output_dir`, stores its transcript and records it Fetched; with no
    /// original-language track, records it Unavailable once it is at least 7
    /// days old. Best effort: logs and swallows every failure, leaving the
    /// video Missing.
    fn fetch(&self, video: &Video, output_dir: &Path);

    /// Missing-subtitles recovery pass over `videos`, run by
    /// `InternalVideoReconciler`: schedules a `FetchSubtitles` for each
    /// subtitle-fetchable video not in `skip_ids`. A video that already has a
    /// fetch pending or running gets no second one: `TaskRepository::schedule`
    /// dedupes it.
    fn schedule_missing(
        &self,
        videos: &[Video],
        skip_ids: &HashSet<&VideoRecordId>,
        output_dir: &Path,
    ) -> anyhow::Result<()>;
}

impl SubtitleFetcherApi for SubtitleFetcher {
    fn fetch(&self, _video: &Video, _output_dir: &Path) {}

    fn schedule_missing(
        &self,
        _videos: &[Video],
        _skip_ids: &HashSet<&VideoRecordId>,
        _output_dir: &Path,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
