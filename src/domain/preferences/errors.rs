use crate::domain::shared::ValidationError;
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
pub enum UpdatePreferencesError {
    Invalid(ValidationError),
    Repository(anyhow::Error),
}

impl fmt::Display for UpdatePreferencesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(e) => write!(f, "{e}"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for UpdatePreferencesError {}
