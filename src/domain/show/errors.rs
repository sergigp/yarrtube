use crate::domain::playlist::PlaylistPath;
use std::fmt;

#[derive(Debug)]
pub enum MoveShowError {
    NotFound,
    Reconciling,
    WorkInProgress,
    SamePath,
    DestinationInUse(PlaylistPath),
    DestinationExists(PlaylistPath),
    Filesystem(anyhow::Error),
    Repository(anyhow::Error),
}

impl fmt::Display for MoveShowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "playlist or channel not found"),
            Self::Reconciling => write!(
                f,
                "playlist or channel is being synced, try again once it finishes"
            ),
            Self::WorkInProgress => write!(
                f,
                "playlist or channel still has work in progress, try again once its downloads finish"
            ),
            Self::SamePath => write!(f, "playlist or channel is already stored at that path"),
            Self::DestinationInUse(path) => write!(
                f,
                "path \"{path}\" overlaps the path of a playlist or channel"
            ),
            Self::DestinationExists(path) => write!(f, "path \"{path}\" already exists on disk"),
            Self::Filesystem(e) => write!(f, "{e}"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for MoveShowError {}
