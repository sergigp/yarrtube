use super::video::Video;
use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::{PlaylistId, PlaylistPath};

/// The playlist or channel that tracks a `RecentVideo`. A channel source
/// additionally carries its channel's avatar filename (when recorded), so
/// listings can render it without an extra per-video lookup — a playlist
/// source has no avatar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoSource {
    Playlist(PlaylistId, PlaylistPath),
    Channel(ChannelHandle, PlaylistPath, Option<String>),
}

/// A `Video` paired with the source that tracks it, for read models (like
/// `VideoSearcher::list_recent`) that combine videos across playlists and
/// channels and need to report which source found each one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentVideo {
    pub video: Video,
    pub source: VideoSource,
}
