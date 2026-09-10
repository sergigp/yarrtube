use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistIdError(pub String);

impl fmt::Display for PlaylistIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PlaylistIdError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoIdError(pub String);

impl fmt::Display for VideoIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for VideoIdError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityError(pub String);

impl fmt::Display for QualityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for QualityError {}
