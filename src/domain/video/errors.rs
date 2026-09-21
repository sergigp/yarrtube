use crate::domain::channel::ChannelHandle;
use crate::domain::shared::PlaylistId;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoError(pub String);

impl fmt::Display for VideoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for VideoError {}

#[derive(Debug)]
pub enum ListVideosError {
    PlaylistNotFound(PlaylistId),
    ChannelNotFound(ChannelHandle),
    Repository(anyhow::Error),
}

impl fmt::Display for ListVideosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlaylistNotFound(id) => write!(f, "playlist {id} not found"),
            Self::ChannelNotFound(id) => write!(f, "channel {id} not found"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ListVideosError {}
