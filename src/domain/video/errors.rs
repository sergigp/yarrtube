use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoError(pub String);

impl fmt::Display for VideoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for VideoError {}
