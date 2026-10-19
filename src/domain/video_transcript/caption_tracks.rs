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
    /// automatic `<lang>` when no automatic `*-orig` exists, trying the
    /// reported language then its base language (`es` for `es-US`); else the
    /// only automatic `*-orig`, with its own language (the reported one was
    /// wrong). No language reported: the only automatic `*-orig`, else the
    /// uploaded track when every uploaded track shares one base language
    /// (the base code preferred).
    pub fn original(&self) -> Option<CaptionTrack> {
        match self.original_language.as_deref() {
            Some(language) => {
                let candidates = language_candidates(language);
                candidates
                    .iter()
                    .find_map(|language| self.manual_in(language))
                    .or_else(|| {
                        candidates
                            .iter()
                            .find_map(|language| self.automatic_in(language))
                    })
                    .or_else(|| self.only_original_automatic())
            }
            None => self
                .only_original_automatic()
                .or_else(|| self.manual_in_only_language()),
        }
    }

    /// The uploaded track in `language`, else a regional or named variant
    /// of it (e.g. `en-US`, `es-ES-7eCR4kqQbL4`), recorded as `language`.
    fn manual_in(&self, language: &str) -> Option<CaptionTrack> {
        let variant_prefix = format!("{language}-");
        self.manual
            .iter()
            .find(|key| key.as_str() == language)
            .or_else(|| {
                self.manual
                    .iter()
                    .find(|key| key.starts_with(&variant_prefix))
            })
            .and_then(|key| track(key, language, CaptionSource::Manual))
    }

    /// The auto-generated track yt-dlp marks as the original in `language`,
    /// else the plain auto-generated one in `language`, but only when no
    /// track is marked as the original: YouTube lists machine translations
    /// of the original under plain language codes.
    fn automatic_in(&self, language: &str) -> Option<CaptionTrack> {
        let original_key = format!("{language}{ORIGINAL_MARKER}");
        let plain_is_original = !self.has_original_automatic();
        self.automatic
            .iter()
            .find(|key| **key == original_key)
            .or_else(|| {
                self.automatic
                    .iter()
                    .find(|key| plain_is_original && key.as_str() == language)
            })
            .and_then(|key| track(key, language, CaptionSource::Automatic))
    }

    fn has_original_automatic(&self) -> bool {
        self.automatic
            .iter()
            .any(|key| key.ends_with(ORIGINAL_MARKER))
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

    /// With no language reported, uploaded tracks all in one base language
    /// (e.g. `en` and `en-GB`) can only be in the original language.
    fn manual_in_only_language(&self) -> Option<CaptionTrack> {
        let mut bases = self.manual.iter().map(|key| base_of(key));
        let base = bases.next()?;
        if bases.all(|other| other == base) {
            self.manual_in(base)
        } else {
            None
        }
    }
}

/// A language code's primary subtag: `en` for `en-GB`.
fn base_of(language: &str) -> &str {
    language.split_once('-').map_or(language, |(base, _)| base)
}

/// The reported language, then its base language when it is regional:
/// YouTube reports e.g. `es-US` for a video whose tracks are `es`.
fn language_candidates(language: &str) -> Vec<&str> {
    match language.split_once('-') {
        Some((base, _)) => vec![language, base],
        None => vec![language],
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
            Some(track_of("en-US", "en", CaptionSource::Manual))
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
    fn it_should_fall_back_to_the_base_of_a_regional_original_language() {
        assert_eq!(
            tracks(Some("es-US"), &[], &["en-orig", "es-orig", "es", "en"]).original(),
            Some(track_of("es-orig", "es", CaptionSource::Automatic))
        );
    }

    #[test]
    fn it_should_never_select_the_live_chat() {
        assert_eq!(
            tracks(Some("en"), &["live_chat"], &["en-orig"]).original(),
            Some(track_of("en-orig", "en", CaptionSource::Automatic))
        );
    }

    #[test]
    fn it_should_select_nothing_when_no_language_is_reported_and_several_tracks_are_original() {
        assert_eq!(tracks(None, &[], &["en-orig", "es-orig"]).original(), None);
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
        assert_eq!(tracks(None, &[], &["en", "fr"]).original(), None);
    }

    #[test]
    fn it_should_select_the_only_original_when_the_reported_language_has_none() {
        assert_eq!(
            tracks(Some("en"), &[], &["es-orig", "es", "en", "fr"]).original(),
            Some(track_of("es-orig", "es", CaptionSource::Automatic))
        );
    }

    #[test]
    fn it_should_select_a_plain_track_when_no_original_is_marked() {
        assert_eq!(
            tracks(Some("yue"), &[], &["yue"]).original(),
            Some(track_of("yue", "yue", CaptionSource::Automatic))
        );
    }

    #[test]
    fn it_should_select_the_uploaded_track_when_no_language_is_reported() {
        assert_eq!(
            [
                tracks(None, &["en-GB", "en"], &[]).original(),
                tracks(None, &["en", "de"], &[]).original(),
            ],
            [Some(track_of("en", "en", CaptionSource::Manual)), None]
        );
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
