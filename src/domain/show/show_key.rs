use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;

/// Identifies a tracked source's show: what moves and reconciles of the same
/// source exclude each other on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShowKey {
    Playlist(PlaylistId),
    Channel(ChannelHandle),
}
