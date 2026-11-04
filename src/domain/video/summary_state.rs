use crate::domain::shared::ValidationError;

/// Where a video stands in getting its AI summary — see the
/// `video-summaries` capability. `Generated` and `Unavailable` are final.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryState {
    Missing,
    Generated,
    Unavailable,
}

impl SummaryState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Missing => "MISSING",
            Self::Generated => "GENERATED",
            Self::Unavailable => "UNAVAILABLE",
        }
    }

    pub fn parse(value: &str) -> Result<Self, ValidationError> {
        match value {
            "MISSING" => Ok(Self::Missing),
            "GENERATED" => Ok(Self::Generated),
            "UNAVAILABLE" => Ok(Self::Unavailable),
            other => Err(ValidationError(format!("unknown summary state '{other}'"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_round_trip_every_state_through_its_string_representation() {
        let states = [
            SummaryState::Missing,
            SummaryState::Generated,
            SummaryState::Unavailable,
        ];

        assert_eq!(
            states.map(|state| SummaryState::parse(state.as_str())),
            states.map(Ok)
        );
    }
}
