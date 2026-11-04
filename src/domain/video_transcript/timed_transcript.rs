use super::transcript_text::{cue_text, is_cue_index, strip_tags, without_first_if};
use super::{TimedLine, TranscriptPart};
use crate::domain::video::VideoChapter;
use std::ops::Range;

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

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The whole transcript as one part when it fits `max_chars`, else cut
    /// at chapter starts, then by size.
    pub fn parts(&self, chapters: &[VideoChapter], max_chars: usize) -> Vec<TranscriptPart> {
        if self.chars(0..self.0.len()) <= max_chars {
            return vec![Self::part(&self.0)];
        }
        let pieces = self
            .sections(chapters)
            .into_iter()
            .flat_map(|section| self.cut_by_size(section, max_chars))
            .collect();
        self.packed(pieces, max_chars)
            .into_iter()
            .map(|range| Self::part(&self.0[range]))
            .collect()
    }

    /// The lines of each chapter, cutting only where one starts.
    fn sections(&self, chapters: &[VideoChapter]) -> Vec<Range<usize>> {
        let mut cuts: Vec<usize> = chapters
            .iter()
            .map(|chapter| {
                self.0
                    .partition_point(|line| line.start_seconds < chapter.start_seconds)
            })
            .filter(|cut| (1..self.0.len()).contains(cut))
            .collect();
        cuts.dedup();
        let starts = std::iter::once(0).chain(cuts.iter().copied());
        let ends = cuts.iter().copied().chain(std::iter::once(self.0.len()));
        starts.zip(ends).map(|(start, end)| start..end).collect()
    }

    /// `range` cut into consecutive ranges of at most `max_chars`. A line
    /// longer than that stays whole, on its own.
    fn cut_by_size(&self, range: Range<usize>, max_chars: usize) -> Vec<Range<usize>> {
        let (mut pieces, start, _) = range.clone().fold(
            (Vec::new(), range.start, 0),
            |(mut pieces, start, chars), end| {
                let line_chars = self.0[end].text.chars().count();
                match end > start && chars + 1 + line_chars > max_chars {
                    true => {
                        pieces.push(start..end);
                        (pieces, end, line_chars)
                    }
                    false => (pieces, start, chars + usize::from(end > start) + line_chars),
                }
            },
        );
        pieces.push(start..range.end);
        pieces
    }

    /// `pieces` with neighbours joined while they fit `max_chars` together.
    fn packed(&self, pieces: Vec<Range<usize>>, max_chars: usize) -> Vec<Range<usize>> {
        pieces.into_iter().fold(Vec::new(), |mut packed, piece| {
            match packed.last_mut() {
                Some(last) if self.chars(last.start..piece.end) <= max_chars => {
                    *last = last.start..piece.end
                }
                _ => packed.push(piece),
            }
            packed
        })
    }

    /// The length of `range` joined by line breaks.
    fn chars(&self, range: Range<usize>) -> usize {
        self.0[range]
            .iter()
            .map(|line| line.text.chars().count() + 1)
            .sum::<usize>()
            .saturating_sub(1)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_keep_a_line_over_the_limit_on_its_own() {
        let transcript = transcript(&[(0, "aaaaaaaaaa"), (5, "bb")]);

        let parts = transcript.parts(&[], 5);

        assert_eq!(parts, vec![part(0, 0, "aaaaaaaaaa"), part(5, 5, "bb")]);
    }

    #[test]
    fn it_should_fill_a_part_up_to_exactly_the_limit() {
        let transcript = transcript(&[(0, "aaaa"), (5, "bbbb"), (10, "c"), (15, "d")]);

        let parts = transcript.parts(&[], 9);

        assert_eq!(parts, vec![part(0, 5, "aaaa\nbbbb"), part(10, 15, "c\nd")]);
    }

    #[test]
    fn it_should_count_line_breaks_in_the_size_of_a_transcript() {
        let transcript = transcript(&[(0, "aaaa"), (5, "bbbb")]);

        let parts = transcript.parts(&[], 8);

        assert_eq!(parts, vec![part(0, 0, "aaaa"), part(5, 5, "bbbb")]);
    }

    fn transcript(lines: &[(i64, &str)]) -> TimedTranscript {
        TimedTranscript(
            lines
                .iter()
                .map(|(start_seconds, text)| TimedLine {
                    start_seconds: *start_seconds,
                    text: text.to_string(),
                })
                .collect(),
        )
    }

    fn part(start_seconds: i64, end_seconds: i64, text: &str) -> TranscriptPart {
        TranscriptPart {
            start_seconds,
            end_seconds,
            text: text.to_string(),
        }
    }
}
