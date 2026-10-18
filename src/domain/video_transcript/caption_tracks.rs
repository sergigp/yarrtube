use super::caption_language::CaptionLanguage;
use super::caption_source::CaptionSource;
use super::caption_track::CaptionTrack;

/// What yt-dlp appends to the key of the auto-generated track in the
/// video's original language, e.g. `en-orig`.
const ORIGINAL_MARKER: &str = "-orig";

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
        match self.original_language.as_deref() {
            Some(language) => self
                .manual_in(language)
                .or_else(|| self.automatic_in(language)),
            None => self.only_original_automatic(),
        }
    }
}

impl CaptionTracks {
    /// The uploaded track in `language`, else a regional variant of it
    /// (e.g. `en-US` for `en`).
    fn manual_in(&self, language: &str) -> Option<CaptionTrack> {
        let regional_prefix = format!("{language}-");
        self.manual
            .iter()
            .find(|key| key.as_str() == language)
            .or_else(|| {
                self.manual
                    .iter()
                    .find(|key| key.starts_with(&regional_prefix))
            })
            .and_then(|key| track(key, key, CaptionSource::Manual))
    }

    /// The auto-generated track yt-dlp marks as the original in `language`,
    /// else the plain auto-generated one in `language`.
    fn automatic_in(&self, language: &str) -> Option<CaptionTrack> {
        let original_key = format!("{language}{ORIGINAL_MARKER}");
        self.automatic
            .iter()
            .find(|key| **key == original_key)
            .or_else(|| self.automatic.iter().find(|key| key.as_str() == language))
            .and_then(|key| track(key, language, CaptionSource::Automatic))
    }

    /// With no language reported, the original is only known when exactly
    /// one auto-generated track is marked as such.
    fn only_original_automatic(&self) -> Option<CaptionTrack> {
        let mut originals = self.automatic.iter().filter_map(|key| {
            key.strip_suffix(ORIGINAL_MARKER)
                .map(|language| (key, language))
        });
        match (originals.next(), originals.next()) {
            (Some((key, language)), None) => track(key, language, CaptionSource::Automatic),
            _ => None,
        }
    }
}

fn track(key: &str, language: &str, source: CaptionSource) -> Option<CaptionTrack> {
    Some(CaptionTrack {
        key: key.to_string(),
        language: CaptionLanguage::new(language).ok()?,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_select_the_uploaded_track_in_the_original_language() {
        assert_eq!(
            tracks(Some("en"), &["fr", "en"], &["en-orig", "en"]).original(),
            Some(track_of("en", "en", CaptionSource::Manual))
        );
    }

    #[test]
    fn it_should_select_a_regional_uploaded_track_in_the_original_language() {
        assert_eq!(
            tracks(Some("en"), &["fr", "en-US"], &["en-orig"]).original(),
            Some(track_of("en-US", "en-US", CaptionSource::Manual))
        );
    }

    #[test]
    fn it_should_prefer_the_exact_uploaded_track_over_a_regional_one() {
        assert_eq!(
            tracks(Some("en"), &["en-GB", "en"], &[]).original(),
            Some(track_of("en", "en", CaptionSource::Manual))
        );
    }

    #[test]
    fn it_should_fall_back_to_the_original_auto_generated_track() {
        assert_eq!(
            tracks(Some("es"), &["en"], &["en", "es-orig", "es"]).original(),
            Some(track_of("es-orig", "es", CaptionSource::Automatic))
        );
    }

    #[test]
    fn it_should_fall_back_to_the_auto_generated_track_in_the_original_language() {
        assert_eq!(
            tracks(Some("es"), &[], &["en", "es"]).original(),
            Some(track_of("es", "es", CaptionSource::Automatic))
        );
    }

    #[test]
    fn it_should_select_nothing_when_only_translations_exist() {
        assert_eq!(tracks(Some("es"), &["en"], &["en", "fr"]).original(), None);
    }

    #[test]
    fn it_should_select_the_only_original_auto_generated_track_when_no_language_is_reported() {
        assert_eq!(
            tracks(None, &["fr"], &["en", "de-orig", "fr"]).original(),
            Some(track_of("de-orig", "de", CaptionSource::Automatic))
        );
    }

    #[test]
    fn it_should_select_nothing_when_no_language_is_reported_and_no_track_is_original() {
        assert_eq!(tracks(None, &["en"], &["en", "fr"]).original(), None);
    }

    fn tracks(language: Option<&str>, manual: &[&str], automatic: &[&str]) -> CaptionTracks {
        CaptionTracks {
            original_language: language.map(str::to_string),
            manual: manual.iter().map(|key| key.to_string()).collect(),
            automatic: automatic.iter().map(|key| key.to_string()).collect(),
        }
    }

    fn track_of(key: &str, language: &str, source: CaptionSource) -> CaptionTrack {
        CaptionTrack {
            key: key.to_string(),
            language: CaptionLanguage::new(language).unwrap(),
            source,
        }
    }
}
