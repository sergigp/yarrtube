use std::fmt;

#[derive(Debug)]
pub enum ReadPreferencesError {
    Repository(anyhow::Error),
}

impl fmt::Display for ReadPreferencesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ReadPreferencesError {}

#[derive(Debug)]
pub enum SavePreferencesError {
    Repository(anyhow::Error),
}

impl fmt::Display for SavePreferencesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SavePreferencesError {}
