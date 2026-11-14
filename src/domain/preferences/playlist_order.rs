use crate::domain::shared::ValidationError;

/// The order a category's Plex playlist lists its episodes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaylistOrder {
    #[default]
    NewestFirst,
    OldestFirst,
}

impl PlaylistOrder {
    pub fn new(_value: &str) -> Result<Self, ValidationError> {
        Ok(Self::NewestFirst)
    }

    pub fn as_str(&self) -> &'static str {
        "newest_first"
    }
}
