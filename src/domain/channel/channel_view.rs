use super::channel_handle::ChannelHandle;
use crate::domain::playlist::PlaylistPath;

/// A channel as listed, with its count of downloaded videos not yet watched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelView {
    pub id: ChannelHandle,
    pub name: String,
    pub path: PlaylistPath,
    pub avatar_filename: Option<String>,
    pub unwatched_count: usize,
}
