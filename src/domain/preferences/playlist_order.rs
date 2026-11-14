use crate::domain::shared::ValidationError;

/// The order a category's Plex playlist lists its episodes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaylistOrder {
    #[default]
    NewestFirst,
    OldestFirst,
}

impl PlaylistOrder {
    pub fn new(value: &str) -> Result<Self, ValidationError> {
        match value {
            "newest_first" => Ok(Self::NewestFirst),
            "oldest_first" => Ok(Self::OldestFirst),
            _ => Err(ValidationError(format!(
                "Playlist order must be \"newest_first\" or \"oldest_first\" (got \"{value}\")"
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NewestFirst => "newest_first",
            Self::OldestFirst => "oldest_first",
        }
    }
}
