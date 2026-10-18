use crate::domain::shared::ValidationError;

/// Where a video stands in getting its original-language subtitles — see
/// the `video-subtitles` capability. `Unavailable` is terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtitleState {
    Missing,
    Fetched,
    Unavailable,
}

impl SubtitleState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Missing => "MISSING",
            Self::Fetched => "FETCHED",
            Self::Unavailable => "UNAVAILABLE",
        }
    }

    pub fn parse(value: &str) -> Result<Self, ValidationError> {
        match value {
            "MISSING" => Ok(Self::Missing),
            "FETCHED" => Ok(Self::Fetched),
            "UNAVAILABLE" => Ok(Self::Unavailable),
            other => Err(ValidationError(format!("unknown subtitle state '{other}'"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_round_trip_every_state_through_its_string_representation() {
        let states = [
            SubtitleState::Missing,
            SubtitleState::Fetched,
            SubtitleState::Unavailable,
        ];

        assert_eq!(
            states.map(|state| SubtitleState::parse(state.as_str())),
            states.map(Ok)
        );
    }

    #[test]
    fn it_should_reject_an_unknown_state() {
        assert_eq!(
            SubtitleState::parse("BOGUS"),
            Err(ValidationError(
                "unknown subtitle state 'BOGUS'".to_string()
            ))
        );
    }
}
