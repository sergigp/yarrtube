use crate::domain::category::CategoryId;
use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::video::VideoId;
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
    CategoryNotFound(CategoryId),
    Repository(anyhow::Error),
}

impl fmt::Display for ListVideosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlaylistNotFound(id) => write!(f, "playlist {id} not found"),
            Self::ChannelNotFound(id) => write!(f, "channel {id} not found"),
            Self::CategoryNotFound(id) => write!(f, "category {id} not found"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ListVideosError {}

#[derive(Debug)]
pub enum UpdateWatchStateError {
    VideoNotFound(VideoId),
    VideoNotDownloaded(VideoId),
    ChannelNotFound(ChannelHandle),
    CategoryNotFound(CategoryId),
    Repository(anyhow::Error),
}

impl fmt::Display for UpdateWatchStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VideoNotFound(id) => write!(f, "video {id} not found"),
            Self::VideoNotDownloaded(id) => write!(f, "video {id} is not downloaded"),
            Self::ChannelNotFound(id) => write!(f, "channel {id} not found"),
            Self::CategoryNotFound(id) => write!(f, "category {id} not found"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for UpdateWatchStateError {}

#[derive(Debug)]
pub enum ReadSubtitlesError {
    PlaylistNotFound(PlaylistId),
    ChannelNotFound(ChannelHandle),
    VideoNotFound(VideoId),
    SubtitlesNotFound(VideoId),
    Repository(anyhow::Error),
}

impl fmt::Display for ReadSubtitlesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlaylistNotFound(id) => write!(f, "playlist {id} not found"),
            Self::ChannelNotFound(id) => write!(f, "channel {id} not found"),
            Self::VideoNotFound(id) => write!(f, "video {id} not found"),
            Self::SubtitlesNotFound(id) => write!(f, "video {id} has no subtitles"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ReadSubtitlesError {}
