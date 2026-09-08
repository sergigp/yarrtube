use super::youtube_playlist_id::YoutubePlaylistId;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistError(pub String);

impl fmt::Display for PlaylistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PlaylistError {}

#[derive(Debug)]
pub enum CreatePlaylistError {
    YoutubePlaylistNotFound(YoutubePlaylistId),
    Lookup(anyhow::Error),
    Repository(anyhow::Error),
}

impl fmt::Display for CreatePlaylistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::YoutubePlaylistNotFound(id) => {
                write!(
                    f,
                    "YouTube playlist {id} does not exist or is not accessible"
                )
            }
            Self::Lookup(e) => write!(f, "{e}"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CreatePlaylistError {}

#[derive(Debug)]
pub enum DeletePlaylistError {
    NotFound(YoutubePlaylistId),
    Repository(anyhow::Error),
}

impl fmt::Display for DeletePlaylistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "playlist {id} not found"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DeletePlaylistError {}
