use super::errors::VideoError;

/// The only status this change ever writes. More statuses (e.g. downloaded)
/// arrive once something consumes `PENDING`, which is out of scope here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoStatus {
    Pending,
}

impl VideoStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
        }
    }

    pub fn parse(s: &str) -> Result<Self, VideoError> {
        match s {
            "PENDING" => Ok(Self::Pending),
            other => Err(VideoError(format!("unknown video status '{other}'"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_round_trip_through_its_string_representation() {
        assert_eq!(
            VideoStatus::parse(VideoStatus::Pending.as_str()).unwrap(),
            VideoStatus::Pending
        );
    }

    #[test]
    fn it_should_reject_an_unknown_status() {
        assert!(VideoStatus::parse("DOWNLOADED").is_err());
    }
}
