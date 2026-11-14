use crate::domain::shared::ValidationError;

/// Whether a category's Plex playlist keeps watched episodes or drops them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaylistWatched {
    #[default]
    Keep,
    Drop,
}

impl PlaylistWatched {
    pub fn new(value: &str) -> Result<Self, ValidationError> {
        match value {
            "keep" => Ok(Self::Keep),
            "drop" => Ok(Self::Drop),
            _ => Err(ValidationError(format!(
                "Watched episodes in playlists must be \"keep\" or \"drop\" (got \"{value}\")"
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Drop => "drop",
        }
    }
}
