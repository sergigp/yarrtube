const HEADER: &str = "WEBVTT";

/// A video's subtitles in WebVTT, the only caption format browsers play,
/// converted from the SubRip file stored next to the episode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebVtt(String);

impl WebVtt {
    pub fn from_srt(srt: &str) -> Self {
        let cues = srt
            .trim_start_matches('\u{feff}')
            .split('\n')
            .map(|line| to_vtt_line(line.strip_suffix('\r').unwrap_or(line)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_convert_srt_timings_to_web_vtt() {
        assert_eq!(
            WebVtt::from_srt(
                "1\n00:00:01,500 --> 00:00:03,000\nHello, there\n\n2\n01:02:03,004 --> 01:02:05,000\nGeneral Kenobi\n"
            )
            .as_str(),
            "WEBVTT\n\n1\n00:00:01.500 --> 00:00:03.000\nHello, there\n\n2\n01:02:03.004 --> 01:02:05.000\nGeneral Kenobi\n"
        );
    }

    #[test]
    fn it_should_strip_a_bom_and_normalise_crlf() {
        assert_eq!(
            WebVtt::from_srt("\u{feff}1\r\n00:00:01,500 --> 00:00:03,000\r\nHello there\r\n")
                .as_str(),
            "WEBVTT\n\n1\n00:00:01.500 --> 00:00:03.000\nHello there\n"
        );
    }
}
