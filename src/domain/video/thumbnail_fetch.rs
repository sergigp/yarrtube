use super::fetched_thumbnail::FetchedThumbnail;

/// A thumbnail-only fetch's outcome: the thumbnail was written, or none could
/// be obtained, carrying `yt-dlp`'s reported error text when it reported one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbnailFetch {
    Fetched(FetchedThumbnail),
    /// Clean yt-dlp failure, or success that printed no thumbnail.
    Unavailable {
        reason: Option<String>,
    },
}
