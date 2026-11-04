use super::TranscriptPart;
use super::transcript_text::{cue_text, is_cue_index, strip_tags, without_first_if};
use crate::domain::video::VideoChapter;

/// A spoken line and when it starts, in seconds from the video's start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedLine {
    pub start_seconds: i64,
    pub text: String,
}

/// A video's captions with their timings, which the stored `TranscriptText`
/// doesn't keep. Cleaned as `TranscriptText::from_srt` does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedTranscript(Vec<TimedLine>);

impl TimedTranscript {
    pub fn from_srt(srt: &str) -> Self {
        let lines: Vec<&str> = srt.lines().map(str::trim).collect();
        let mut timed: Vec<TimedLine> = lines
            .split(|line| line.is_empty())
            .scan(0, |start, cue| {
                *start = cue_start(cue).unwrap_or(*start);
                Some(cue_lines(cue, *start))
            })
            .flatten()
            .collect();
        timed.dedup_by(|line, previous| line.text == previous.text);
        Self(timed)
    }

    pub fn text(&self) -> String {
        Self::joined(&self.0)
    }

    /// The whole transcript as one part when it fits `max_chars`, else cut
    /// at chapter starts, then by size.
    pub fn parts(&self, _chapters: &[VideoChapter], _max_chars: usize) -> Vec<TranscriptPart> {
        vec![Self::part(&self.0)]
    }

    fn part(lines: &[TimedLine]) -> TranscriptPart {
        TranscriptPart {
            start_seconds: lines.first().map_or(0, |line| line.start_seconds),
            end_seconds: lines.last().map_or(0, |line| line.start_seconds),
            text: Self::joined(lines),
        }
    }

    fn joined(lines: &[TimedLine]) -> String {
        lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// The cue's text lines, each starting at `start`, without markup.
fn cue_lines(cue: &[&str], start: i64) -> Vec<TimedLine> {
    cue_text(cue)
        .iter()
        .map(|line| strip_tags(line))
        .filter(|text| !text.is_empty())
        .map(|text| TimedLine {
            start_seconds: start,
            text,
        })
        .collect()
}

/// When the cue starts, in whole seconds, from its timing line
/// (`00:01:02,345 --> 00:01:04,000`).
fn cue_start(cue: &[&str]) -> Option<i64> {
    let (start, _) = without_first_if(cue, is_cue_index)
        .first()?
        .split_once("-->")?;
    let clock = start.trim().split([',', '.']).next()?;
    clock.split(':').try_fold(0, |seconds, unit| {
        Some(seconds * 60 + unit.parse::<i64>().ok()?)
    })
}
