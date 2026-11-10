use super::errors::VideoError;

/// A video's position in its download lifecycle: stored, then downloaded via
/// `yt-dlp`, tracking success or failure (with retries, see the `video-retries`
/// capability) along the way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoStatus {
    Pending,
    InProgress,
    Downloaded,
    Retrying,
    Failed,
    Unavailable,
}

impl VideoStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::InProgress => "IN_PROGRESS",
            Self::Downloaded => "DOWNLOADED",
            Self::Retrying => "RETRYING",
            Self::Failed => "FAILED",
            Self::Unavailable => "UNAVAILABLE",
        }
    }

    pub fn parse(s: &str) -> Result<Self, VideoError> {
        match s {
            "PENDING" => Ok(Self::Pending),
            "IN_PROGRESS" => Ok(Self::InProgress),
            "DOWNLOADED" => Ok(Self::Downloaded),
            "RETRYING" => Ok(Self::Retrying),
            "FAILED" => Ok(Self::Failed),
            "UNAVAILABLE" => Ok(Self::Unavailable),
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
    fn it_should_round_trip_retrying_through_its_string_representation() {
        assert_eq!(
            VideoStatus::parse(VideoStatus::Retrying.as_str()).unwrap(),
            VideoStatus::Retrying
        );
    }

    #[test]
    fn it_should_round_trip_failed_through_its_string_representation() {
        assert_eq!(
            VideoStatus::parse(VideoStatus::Failed.as_str()).unwrap(),
            VideoStatus::Failed
        );
    }

    #[test]
    fn it_should_round_trip_unavailable_through_its_string_representation() {
        assert_eq!(
            VideoStatus::parse(VideoStatus::Unavailable.as_str()).unwrap(),
            VideoStatus::Unavailable
        );
    }

    #[test]
    fn it_should_reject_an_unknown_status() {
        assert!(VideoStatus::parse("BOGUS").is_err());
    }
}
