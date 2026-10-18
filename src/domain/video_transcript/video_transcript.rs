use super::caption_language::CaptionLanguage;
use super::caption_source::CaptionSource;
use super::caption_track::CaptionTrack;
use super::transcript_text::TranscriptText;
use crate::domain::video::VideoRecordId;
use chrono::{DateTime, Utc};

/// What a video says, as text: the plain-text transcript of its
/// original-language captions — see the `video-subtitles` capability. At
/// most one per video.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoTranscript {
    pub video_id: VideoRecordId,
    pub language: CaptionLanguage,
    pub source: CaptionSource,
    pub text: TranscriptText,
    pub created_at: DateTime<Utc>,
}

impl VideoTranscript {
    pub fn create(
        video_id: VideoRecordId,
        track: &CaptionTrack,
        text: TranscriptText,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            video_id,
            language: track.language.clone(),
            source: track.source,
            text,
            created_at: now,
        }
    }
}
