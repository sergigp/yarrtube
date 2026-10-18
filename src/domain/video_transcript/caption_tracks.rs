use super::caption_language::CaptionLanguage;
use super::caption_source::CaptionSource;
use super::caption_track::CaptionTrack;

/// The caption tracks yt-dlp's info JSON lists for a video.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CaptionTracks {
    /// The info JSON's `language`: the video's original language, when
    /// reported.
    pub original_language: Option<String>,
    /// Keys of `subtitles`: tracks uploaded by the creator.
    pub manual: Vec<String>,
    /// Keys of `automatic_captions`: YouTube's auto-generated tracks,
    /// translations included.
    pub automatic: Vec<String>,
}

impl CaptionTracks {
    /// Manual `<lang>`/`<lang>-*`, else automatic `<lang>-orig`, else
    /// automatic `<lang>`; no language reported: the only automatic `*-orig`.
    pub fn original(&self) -> Option<CaptionTrack> {
        let language = self.original_language.as_deref()?;
        self.manual
            .iter()
            .find(|key| key.as_str() == language)
            .and_then(|key| track(key, language, CaptionSource::Manual))
    }
}

fn track(key: &str, language: &str, source: CaptionSource) -> Option<CaptionTrack> {
    Some(CaptionTrack {
        key: key.to_string(),
        language: CaptionLanguage::new(language).ok()?,
        source,
    })
}
