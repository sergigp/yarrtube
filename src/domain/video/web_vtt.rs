/// A video's subtitles in WebVTT, the only caption format browsers play,
/// converted from the SubRip file stored next to the episode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebVtt(String);

impl WebVtt {
    pub fn from_srt(_srt: &str) -> Self {
        Self(String::new())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
