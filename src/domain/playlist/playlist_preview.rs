use super::playlist_id::PlaylistId;
use super::playlist_name::PlaylistName;

/// What YouTube reports about a playlist before it is tracked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistPreview {
    pub id: PlaylistId,
    pub name: PlaylistName,
    pub video_count: u64,
}
