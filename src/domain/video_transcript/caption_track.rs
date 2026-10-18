use super::caption_language::CaptionLanguage;
use super::caption_source::CaptionSource;

/// The one caption track a subtitle fetch downloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionTrack {
    /// yt-dlp's `--sub-langs` key, e.g. `en-orig`.
    pub key: String,
    /// The track's language without the `-orig` marker, e.g. `en`.
    pub language: CaptionLanguage,
    pub source: CaptionSource,
}
