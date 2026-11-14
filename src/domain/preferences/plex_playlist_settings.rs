use super::{PlaylistOrder, PlaylistWatched};

/// How every category's Plex smart playlist lists its episodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlexPlaylistSettings {
    pub watched: PlaylistWatched,
    pub order: PlaylistOrder,
}
