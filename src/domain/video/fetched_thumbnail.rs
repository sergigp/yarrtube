use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// A thumbnail-only fetch's result: the staged thumbnail's path and the
/// video's publish time, when `yt-dlp` reported one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedThumbnail {
    pub path: PathBuf,
    pub published_at: Option<DateTime<Utc>>,
}
