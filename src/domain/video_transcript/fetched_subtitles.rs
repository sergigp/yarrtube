use std::path::PathBuf;

/// A caption-only download's result: the staged `.srt` and its contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedSubtitles {
    pub path: PathBuf,
    pub srt: String,
}
