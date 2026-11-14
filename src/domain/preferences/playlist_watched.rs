use crate::domain::shared::ValidationError;

/// Whether a category's Plex playlist keeps watched episodes or drops them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaylistWatched {
    #[default]
    Keep,
    Drop,
}

impl PlaylistWatched {
    pub fn new(_value: &str) -> Result<Self, ValidationError> {
        Ok(Self::Keep)
    }

    pub fn as_str(&self) -> &'static str {
        "keep"
    }
}
