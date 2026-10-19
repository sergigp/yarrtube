use crate::domain::event::DomainEvent;
use crate::domain::task::Task;
use crate::domain::video::{
    EpisodePaths, Video, VideoRecordId, VideoSubtitlesFetched, subtitle_staging_dir,
};
use crate::domain::video_transcript::{CaptionTrack, TranscriptText, VideoTranscript};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::sqlite_video_transcript_repository::VideoTranscriptRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::{
    CaptionProbe, VideoDownloaderRepository,
};
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::Duration;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tracing::warn;

/// How long both since its publish time and since its first check without
/// captions a video with no original-language captions waits before its
/// subtitles are recorded Unavailable.
const UNAVAILABLE_AFTER: Duration = Duration::days(7);

/// Best-effort fetch of a downloaded video's original-language subtitles —
/// see the `video-subtitles` capability. The fetch itself runs from the
/// `fetch_subtitles` task and never returns an error to its caller; it is
/// scheduled by `FetchSubtitlesOnVideoDownloaded` after a download and by
/// the reconcilers' missing-subtitles recovery.
#[derive(Clone)]
pub struct SubtitleFetcher {
    video_repository: Arc<dyn VideoRepository>,
    video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    task_repository: Arc<dyn TaskRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl SubtitleFetcher {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        task_repository: Arc<dyn TaskRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_repository,
            video_transcript_repository,
            video_downloader_repository,
            video_file_repository,
            task_repository,
            event_publisher,
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
    /// original-language track and no yt-dlp warning, records the first such
    /// check and records it Unavailable once both its publish time and that
    /// first check are at least 7 days old. Best effort: logs and swallows
    /// every failure, leaving the video Missing. Publishes
    /// `VideoSubtitlesFetched` once Fetched.
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
        if !video.is_subtitle_fetchable() {
            return;
        }
        if let Err(e) = self.fetch_and_place(video, output_dir) {
            warn!(video_id = %video.id, error = %e, "failed to fetch video subtitles");
        }
        self.discard_staging(video);
    }

    fn schedule_missing(
        &self,
        videos: &[Video],
        skip_ids: &HashSet<&VideoRecordId>,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        videos
            .iter()
            .filter(|v| v.is_subtitle_fetchable() && !skip_ids.contains(&v.id))
            .try_for_each(|video| self.schedule_fetch(video, output_dir))
    }
}

impl SubtitleFetcher {
    fn schedule_fetch(&self, video: &Video, output_dir: &Path) -> anyhow::Result<()> {
        self.task_repository.schedule(
            &Task::FetchSubtitles {
                video_id: video.id.as_str().to_string(),
                output_dir: output_dir.to_string_lossy().to_string(),
            },
            self.clock.now(),
        )
    }

    fn fetch_and_place(&self, video: &Video, output_dir: &Path) -> anyhow::Result<()> {
        let Some(filename) = video.filename.as_deref() else {
            return Ok(());
        };
        let paths = EpisodePaths::from_media(filename);
        let staging = subtitle_staging_dir(&self.videos_path, &video.id);
        let probe = self
            .video_downloader_repository
            .probe_captions(&video.youtube_id.to_url(), &staging)?;
        match probe.tracks.original() {
            Some(track) => {
                self.place_subtitles(video, &paths, &probe, &track, &staging, output_dir)
            }
            None if !probe.warnings.is_empty() => {
                Self::log_inconclusive(video, &probe);
                Ok(())
            }
            None => self.mark_unavailable_if_long_missing(video, &probe),
        }
    }

    /// yt-dlp can withhold caption tracks silently (e.g. without a PO
    /// token) and only says so in a warning: a probe without a selectable
    /// track that warned proves nothing, so the video stays Missing. The
    /// probe already dropped warnings that can't affect captions.
    fn log_inconclusive(video: &Video, probe: &CaptionProbe) {
        warn!(
            video_id = %video.id,
            warnings = ?probe.warnings,
            "caption probe warned and found no original-language track"
        );
    }

    /// Records the first check without captions, if none is recorded yet.
    /// Captions often appear days after an upload, and a playlist added
    /// today may hold old videos: only a video published and first checked
    /// without captions at least `UNAVAILABLE_AFTER` ago is given up on,
    /// any other stays Missing for a later reconcile pass to try again.
    fn mark_unavailable_if_long_missing(
        &self,
        video: &Video,
        probe: &CaptionProbe,
    ) -> anyhow::Result<()> {
        let now = self.clock.now();
        let video = video.clone().mark_subtitles_first_missing(now);
        let cutoff = now - UNAVAILABLE_AFTER;
        let published_at = probe.published_at.unwrap_or(video.created_at);
        let first_missing_at = video.subtitles_first_missing_at.unwrap_or(now);
        if published_at > cutoff || first_missing_at > cutoff {
            return self.video_repository.update_subtitles(&video);
        }
        self.video_repository
            .update_subtitles(&video.mark_subtitles_unavailable(now))
    }

    /// Downloads `track`, moves it next to the episode, stores its
    /// transcript, then records only the subtitle fields: a download of the
    /// same video may have changed it since `video` was read. The video (or
    /// its whole playlist/channel) may have been deleted, reset for
    /// redownload or redownloaded under another name while the track
    /// downloaded: then nothing is placed, `fetch` discards the staging dir.
    fn place_subtitles(
        &self,
        video: &Video,
        paths: &EpisodePaths,
        probe: &CaptionProbe,
        track: &CaptionTrack,
        staging: &Path,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let fetched = self
            .video_downloader_repository
            .fetch_subtitles(probe, track, staging)?;
        if !self.is_still_downloaded_as(video)? {
            return Ok(());
        }
        let subtitle_filename = paths.subtitle(&track.language);
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
            .update_subtitles(&video.clone().with_subtitles(subtitle_filename, now))?;
        self.publish_fetched(video, output_dir);
        Ok(())
    }

    /// Whether the stored video is still Downloaded as the same file the
    /// fetch started with, so the subtitles still belong next to it.
    fn is_still_downloaded_as(&self, video: &Video) -> anyhow::Result<bool> {
        Ok(self
            .video_repository
            .find(&video.id)?
            .is_some_and(|stored| stored.is_downloaded() && stored.filename == video.filename))
    }

    /// Best-effort, like `VideoDownloaded`: the subtitles are already
    /// recorded, only the Plex rescan is lost.
    fn publish_fetched(&self, video: &Video, output_dir: &Path) {
        let event = DomainEvent::VideoSubtitlesFetched(VideoSubtitlesFetched {
            video_id: video.id.as_str().to_string(),
            output_dir: output_dir.to_string_lossy().to_string(),
        });
        if let Err(e) = self.event_publisher.publish(&event) {
            warn!(video_id = %video.id, error = %e, "failed to publish that the video's subtitles were fetched");
        }
    }

    /// Best-effort: a leftover staging dir is harmless, Plex and Kodi ignore
    /// the staging root.
    fn discard_staging(&self, video: &Video) {
        let dir = subtitle_staging_dir(&self.videos_path, &video.id);
        if let Err(e) = self.video_file_repository.delete_dir_recursive(&dir) {
            warn!(video_id = %video.id, error = %e, "failed to discard the subtitle staging dir");
        }
    }
}
