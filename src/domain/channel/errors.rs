use super::channel_handle::ChannelHandle;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelHandleError(pub String);

impl fmt::Display for ChannelHandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ChannelHandleError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoLimitError(pub String);

impl fmt::Display for VideoLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for VideoLimitError {}

#[derive(Debug)]
pub enum CreateChannelError {
    YoutubeChannelNotFound(ChannelHandle),
    Lookup(anyhow::Error),
    Repository(anyhow::Error),
}

impl fmt::Display for CreateChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::YoutubeChannelNotFound(handle) => {
                write!(
                    f,
                    "YouTube channel {handle} does not exist or is not accessible"
                )
            }
            Self::Lookup(e) => write!(f, "{e}"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CreateChannelError {}

#[derive(Debug)]
pub enum DeleteChannelError {
    NotFound(ChannelHandle),
    Repository(anyhow::Error),
}

impl fmt::Display for DeleteChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(handle) => write!(f, "channel {handle} not found"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DeleteChannelError {}
