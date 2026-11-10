use crate::domain::shared::ValidationError;

/// Where one per-video step stands — see the `video-retries` capability.
/// `Unavailable` (YouTube doesn't have it) is final; `Failed` (gave up after
/// `MAX_ATTEMPTS`) lasts until the user retries the video.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Retrying,
    Done,
    Unavailable,
    Failed,
}

impl StepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Retrying => "RETRYING",
            Self::Done => "DONE",
            Self::Unavailable => "UNAVAILABLE",
            Self::Failed => "FAILED",
        }
    }

    pub fn parse(value: &str) -> Result<Self, ValidationError> {
        match value {
            "PENDING" => Ok(Self::Pending),
            "RETRYING" => Ok(Self::Retrying),
            "DONE" => Ok(Self::Done),
            "UNAVAILABLE" => Ok(Self::Unavailable),
            "FAILED" => Ok(Self::Failed),
            other => Err(ValidationError(format!("unknown step status '{other}'"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_round_trip_every_status_through_its_string_representation() {
        let statuses = [
            StepStatus::Pending,
            StepStatus::Retrying,
            StepStatus::Done,
            StepStatus::Unavailable,
            StepStatus::Failed,
        ];

        assert_eq!(
            statuses.map(|status| StepStatus::parse(status.as_str())),
            statuses.map(Ok)
        );
    }

    #[test]
    fn it_should_reject_an_unknown_status() {
        assert_eq!(
            StepStatus::parse("BOGUS"),
            Err(ValidationError("unknown step status 'BOGUS'".to_string()))
        );
    }
}
