const HEADER: &str = "WEBVTT";

/// A video's subtitles in WebVTT, the only caption format browsers play,
/// converted from the SubRip file stored next to the episode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebVtt(String);

impl WebVtt {
    pub fn from_srt(srt: &str) -> Self {
        let cues = srt
            .split('\n')
            .map(to_vtt_line)
            .collect::<Vec<_>>()
            .join("\n");
        Self(format!("{HEADER}\n\n{cues}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A timing line with SubRip's `,` millisecond separators turned into
/// WebVTT's `.`; any other line unchanged.
fn to_vtt_line(line: &str) -> String {
    if line.contains("-->") {
        line.replace(',', ".")
    } else {
        line.to_string()
    }
}
