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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_parse_keep() {
        assert_eq!(PlaylistWatched::new("keep"), Ok(PlaylistWatched::Keep));
    }

    #[test]
    fn it_should_parse_drop() {
        assert_eq!(PlaylistWatched::new("drop"), Ok(PlaylistWatched::Drop));
    }

    #[test]
    fn it_should_reject_any_other_value() {
        assert_eq!(
            PlaylistWatched::new("Drop"),
            Err(ValidationError(
                "Watched episodes in playlists must be \"keep\" or \"drop\" (got \"Drop\")"
                    .to_string()
            ))
        );
    }

    #[test]
    fn it_should_round_trip_through_its_string() {
        assert_eq!(
            PlaylistWatched::new(PlaylistWatched::Drop.as_str()),
            Ok(PlaylistWatched::Drop)
        );
    }
}
