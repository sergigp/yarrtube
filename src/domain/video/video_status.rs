use super::errors::VideoError;

/// A video's position in its download lifecycle: stored, then downloaded via
/// `yt-dlp`, tracking success or failure (with retries) along the way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoStatus {
    Pending,
    InProgress,
    Downloaded,
    ErroredRetrying,
    Errored,
}

impl VideoStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::InProgress => "IN_PROGRESS",
            Self::Downloaded => "DOWNLOADED",
            Self::ErroredRetrying => "ERRORED_RETRYING",
            Self::Errored => "ERRORED",
        }
    }

    pub fn parse(s: &str) -> Result<Self, VideoError> {
        match s {
            "PENDING" => Ok(Self::Pending),
            "IN_PROGRESS" => Ok(Self::InProgress),
            "DOWNLOADED" => Ok(Self::Downloaded),
            "ERRORED_RETRYING" => Ok(Self::ErroredRetrying),
            "ERRORED" => Ok(Self::Errored),
            other => Err(VideoError(format!("unknown video status '{other}'"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_round_trip_pending_through_its_string_representation() {
        assert_eq!(
            VideoStatus::parse(VideoStatus::Pending.as_str()).unwrap(),
            VideoStatus::Pending
        );
    }

    #[test]
    fn it_should_round_trip_in_progress_through_its_string_representation() {
        assert_eq!(
            VideoStatus::parse(VideoStatus::InProgress.as_str()).unwrap(),
            VideoStatus::InProgress
        );
    }

    #[test]
    fn it_should_round_trip_downloaded_through_its_string_representation() {
        assert_eq!(
            VideoStatus::parse(VideoStatus::Downloaded.as_str()).unwrap(),
            VideoStatus::Downloaded
        );
    }

    #[test]
    fn it_should_round_trip_errored_retrying_through_its_string_representation() {
        assert_eq!(
            VideoStatus::parse(VideoStatus::ErroredRetrying.as_str()).unwrap(),
            VideoStatus::ErroredRetrying
        );
    }

    #[test]
    fn it_should_round_trip_errored_through_its_string_representation() {
        assert_eq!(
            VideoStatus::parse(VideoStatus::Errored.as_str()).unwrap(),
            VideoStatus::Errored
        );
    }

    #[test]
    fn it_should_reject_an_unknown_status() {
        assert!(VideoStatus::parse("BOGUS").is_err());
    }
}
