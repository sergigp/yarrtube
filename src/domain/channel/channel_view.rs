use super::channel_handle::ChannelHandle;
use super::video_limit::VideoLimit;
use crate::domain::playlist::PlaylistPath;
use crate::domain::shared::Quality;

/// A channel as listed, with its count of downloaded videos not yet watched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelView {
    pub id: ChannelHandle,
    pub name: String,
    pub path: PlaylistPath,
    pub quality: Quality,
    pub video_limit: VideoLimit,
    pub avatar_filename: Option<String>,
    pub unwatched_count: usize,
}
