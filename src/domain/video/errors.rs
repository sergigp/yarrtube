use crate::domain::shared::{PlaylistId, VideoId};
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
pub enum AddVideoToCustomPlaylistError {
    PlaylistNotFound(PlaylistId),
    NotCustomPlaylist(PlaylistId),
    YoutubeVideoNotFound(VideoId),
    Lookup(anyhow::Error),
    Repository(anyhow::Error),
}

impl fmt::Display for AddVideoToCustomPlaylistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlaylistNotFound(id) => write!(f, "playlist {id} not found"),
            Self::NotCustomPlaylist(id) => write!(
                f,
                "video membership for playlist {id} cannot be changed manually because it is YouTube-linked"
            ),
            Self::YoutubeVideoNotFound(id) => {
                write!(f, "YouTube video {id} does not exist or is not accessible")
            }
            Self::Lookup(e) => write!(f, "{e}"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AddVideoToCustomPlaylistError {}

#[derive(Debug)]
pub enum RemoveVideoFromPlaylistError {
    PlaylistNotFound(PlaylistId),
    NotCustomPlaylist(PlaylistId),
    VideoNotFound(VideoId),
    Repository(anyhow::Error),
}

impl fmt::Display for RemoveVideoFromPlaylistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlaylistNotFound(id) => write!(f, "playlist {id} not found"),
            Self::NotCustomPlaylist(id) => write!(
                f,
                "video membership for playlist {id} cannot be changed manually because it is YouTube-linked"
            ),
            Self::VideoNotFound(id) => write!(f, "video {id} is not a member of this playlist"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RemoveVideoFromPlaylistError {}
