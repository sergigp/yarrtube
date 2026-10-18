use crate::domain::video::{EpisodePaths, Video, VideoRecordId, subtitle_staging_dir};
use crate::domain::video_transcript::{CaptionTrack, TranscriptText, VideoTranscript};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::sqlite_video_transcript_repository::VideoTranscriptRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::{
    CaptionProbe, VideoDownloaderRepository,
};
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tracing::warn;

/// Best-effort fetch of a downloaded video's original-language subtitles —
/// see the `video-subtitles` capability. The fetch itself runs from the
/// `fetch_subtitles` task and never returns an error to its caller; each
/// reconcile pass's missing-subtitles recovery only schedules it.
#[derive(Clone)]
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
    fn fetch(&self, video: &Video, output_dir: &Path) {
        if let Err(e) = self.fetch_and_place(video, output_dir) {
            warn!(video_id = %video.id, error = %e, "failed to fetch video subtitles");
        }
    }

    fn schedule_missing(
        &self,
        _videos: &[Video],
        _skip_ids: &HashSet<&VideoRecordId>,
        _output_dir: &Path,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl SubtitleFetcher {
    fn fetch_and_place(&self, video: &Video, output_dir: &Path) -> anyhow::Result<()> {
        let staging = subtitle_staging_dir(&self.videos_path, &video.id);
        let probe = self
            .video_downloader_repository
            .probe_captions(&video.youtube_id.to_url(), &staging)?;
        match probe.tracks.original() {
            Some(track) => self.place_subtitles(video, &probe, &track, output_dir),
            None => Ok(()),
        }
    }

    /// Downloads `track`, moves it next to the episode, stores its
    /// transcript, then records only the subtitle fields: a download of the
    /// same video may have changed it since `video` was read.
    fn place_subtitles(
        &self,
        video: &Video,
        probe: &CaptionProbe,
        track: &CaptionTrack,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let Some(number) = video.episode_number else {
            return Ok(());
        };
        let staging = subtitle_staging_dir(&self.videos_path, &video.id);
        let fetched = self
            .video_downloader_repository
            .fetch_subtitles(probe, track, &staging)?;
        let subtitle_filename = EpisodePaths::new(&number, &video.title).subtitle(&track.language);
        self.video_file_repository
            .move_file(&fetched.path, &output_dir.join(&subtitle_filename))?;
        let now = self.clock.now();
        self.video_transcript_repository
            .save(&VideoTranscript::create(
                video.id.clone(),
                track,
                TranscriptText::from_srt(&fetched.srt),
                now,
            ))?;
        self.video_repository
            .update_subtitles(&video.clone().with_subtitles(subtitle_filename, now))
    }
}
