use super::video::Video;
use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::{PlaylistId, PlaylistName, PlaylistPath};

/// The playlist or channel that tracks a `SourcedVideo`, with its display
/// name. A channel source additionally carries its channel's avatar filename
/// (when recorded), so listings can render it without an extra per-video
/// lookup — a playlist source has no avatar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoSource {
    Playlist {
        id: PlaylistId,
        name: PlaylistName,
        path: PlaylistPath,
    },
    Channel {
        handle: ChannelHandle,
        name: String,
        path: PlaylistPath,
        avatar_filename: Option<String>,
    },
}

/// A `Video` paired with the source that tracks it, used by listings that
/// combine videos across playlists and channels (like
/// `VideoSearcher::list_home`) to apply their rules before building the
/// read model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcedVideo {
    pub video: Video,
    pub source: VideoSource,
}
