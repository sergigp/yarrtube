use std::fmt;

#[derive(Debug)]
pub enum RequestPlexSyncError {
    NotConfigured,
    Repository(anyhow::Error),
}

impl fmt::Display for RequestPlexSyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "Plex isn't configured"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RequestPlexSyncError {}
