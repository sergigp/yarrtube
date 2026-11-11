use crate::domain::event::DomainEvent;
use crate::domain::services::{
    VideoOwnerFinder, VideoOwnerFinderApi, VideoUpdater, VideoUpdaterApi,
};
use crate::domain::video::{
    EpisodePaths, LateDataOutcome, Video, VideoId, VideoStepOutcome, VideoSubtitlesFetched,
};
use crate::domain::video_transcript::{
    CaptionProbe, CaptionTrack, TranscriptText, VideoTranscript,
};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::sqlite_video_transcript_repository::VideoTranscriptRepository;
use crate::infrastructure::repositories::youtube_caption_repository::CaptionRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::staging_area::StagingArea;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Best-effort fetch of a downloaded video's original-language subtitles —
/// see the `video-subtitles` capability. The fetch itself runs from the
/// `fetch_subtitles` task and never returns an error to its caller;
/// `VideoReconciler` decides when to schedule it.
#[derive(Clone)]
pub struct SubtitleFetcher {
    video_repository: Arc<dyn VideoRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
    caption_repository: Arc<dyn CaptionRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
    staging_area: Arc<StagingArea>,
    video_updater: VideoUpdater,
}

impl SubtitleFetcher {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
        caption_repository: Arc<dyn CaptionRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
        staging_area: Arc<StagingArea>,
    ) -> Self {
        Self {
            video_updater: VideoUpdater::new(video_repository.clone()),
            video_repository,
            video_owner_finder,
            video_transcript_repository,
            caption_repository,
            video_file_repository,
            event_publisher,
            clock,
            staging_area,
        }
    }
}

pub trait SubtitleFetcherApi: Send + Sync {
    /// No-op when the video, or the playlist or channel holding it (logged),
    /// no longer exists, or the video isn't subtitle-fetchable
    /// (`Video::is_subtitle_fetchable`). Otherwise probes its caption tracks,
    /// downloads the original-language one as `<episode stem>.<lang>.srt` in
    /// its owner's show directory, stores its transcript and records it Done;
    /// otherwise records the check under the `late-video-data` policy (no
    /// track = Incomplete: Unavailable on an old video, Pending on a young
    /// one; a warning or any failure = a failed attempt, re-queued under the
    /// `video-retries` policy). Publishes `VideoSubtitlesFetched` once Done.
    /// Fails only when the video or its owner can't be read or stored.
    fn fetch(&self, video_id: &VideoId) -> anyhow::Result<VideoStepOutcome>;
}

impl SubtitleFetcherApi for SubtitleFetcher {
    fn fetch(&self, video_id: &VideoId) -> anyhow::Result<VideoStepOutcome> {
        let Some(video) = self.video_repository.find(video_id)? else {
            debug!(video_id = %video_id, "video no longer exists, skipping subtitles fetch");
            return Ok(VideoStepOutcome::Settled);
        };
        if !video.is_subtitle_fetchable() {
            return Ok(VideoStepOutcome::Settled);
        }
        let Some(owner) = self.video_owner_finder.find(&video)? else {
            info!(video_id = %video_id, "video has no playlist or channel any more, skipping subtitles fetch");
            return Ok(VideoStepOutcome::Settled);
        };
        let staged = self.staging_area.for_subtitles(&video.id);
        match self.fetch_and_place(&video, staged.path(), &owner.target.output_dir) {
            Ok(()) => Ok(VideoStepOutcome::Settled),
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to fetch video subtitles");
                self.record_failed_fetch(&video, e.to_string())
            }
        }
    }
}

impl SubtitleFetcher {
    fn fetch_and_place(
        &self,
        video: &Video,
        staging: &Path,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let Some(filename) = video.download.filename() else {
            return Ok(());
        };
        let paths = EpisodePaths::from_media(filename);
        let probe = self
            .caption_repository
            .probe_captions(&video.youtube_id.to_url(), staging)?;
        match probe.tracks.original() {
            Some(track) => self.place_subtitles(video, &paths, &probe, &track, staging, output_dir),
            None if !probe.warnings.is_empty() => {
                Self::log_inconclusive(video, &probe);
                Err(anyhow::anyhow!(probe.warnings.join("; ")))
            }
            None => self
                .record_check_on(video, LateDataOutcome::Incomplete, probe.published_at)
                .map(|_| ()),
        }
    }

    /// yt-dlp can withhold caption tracks silently (e.g. without a PO
    /// token) and only says so in a warning: a probe without a selectable
    /// track that warned proves nothing: it counts as a failed check. The
    /// probe already dropped warnings that can't affect captions.
    fn log_inconclusive(video: &Video, probe: &CaptionProbe) {
        warn!(
            video_id = %video.id,
            warnings = ?probe.warnings,
            "caption probe warned and found no original-language track"
        );
    }

    /// The video as stored once the check is recorded on it.
    fn record_check_on(
        &self,
        video: &Video,
        outcome: LateDataOutcome,
        published_at: Option<DateTime<Utc>>,
    ) -> anyhow::Result<Option<Video>> {
        let now = self.clock.now();
        self.video_updater.update(&video.id, |video| {
            let published_at = published_at.unwrap_or(video.created_at);
            Some(video.record_subtitles_check(outcome, published_at, now))
        })
    }

    /// Records a failed fetch, failed for `reason`: Retrying, run again when
    /// its next attempt is due, or Failed.
    fn record_failed_fetch(
        &self,
        video: &Video,
        reason: String,
    ) -> anyhow::Result<VideoStepOutcome> {
        let recorded = self.record_check_on(video, LateDataOutcome::Failed, None)?;
        Ok(VideoStepOutcome::retry_at_or_settled(
            recorded.and_then(|video| video.subtitles.state.next_attempt_at()),
            reason,
        ))
    }

    /// Downloads `track`, moves it next to the episode, stores its
    /// transcript, then records the subtitles on the video as stored now: a
    /// download of the same video may have changed it since `video` was
    /// read. The video (or
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
            .caption_repository
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
        self.video_updater.update(&video.id, |video| {
            Some(video.mark_subtitles_fetched(subtitle_filename.clone(), now))
        })?;
        self.publish_fetched(video, output_dir);
        Ok(())
    }

    /// Whether the stored video is still Downloaded as the same file the
    /// fetch started with, so the subtitles still belong next to it.
    fn is_still_downloaded_as(&self, video: &Video) -> anyhow::Result<bool> {
        Ok(self
            .video_repository
            .find(&video.id)?
            .is_some_and(|stored| {
                stored.download.is_downloaded()
                    && stored.download.filename() == video.download.filename()
            }))
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
}
