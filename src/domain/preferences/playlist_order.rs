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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_parse_newest_first() {
        assert_eq!(
            PlaylistOrder::new("newest_first"),
            Ok(PlaylistOrder::NewestFirst)
        );
    }

    #[test]
    fn it_should_parse_oldest_first() {
        assert_eq!(
            PlaylistOrder::new("oldest_first"),
            Ok(PlaylistOrder::OldestFirst)
        );
    }

    #[test]
    fn it_should_reject_any_other_value() {
        assert_eq!(
            PlaylistOrder::new("newest"),
            Err(ValidationError(
                "Playlist order must be \"newest_first\" or \"oldest_first\" (got \"newest\")"
                    .to_string()
            ))
        );
    }

    #[test]
    fn it_should_round_trip_through_its_string() {
        assert_eq!(
            PlaylistOrder::new(PlaylistOrder::OldestFirst.as_str()),
            Ok(PlaylistOrder::OldestFirst)
        );
    }
}
