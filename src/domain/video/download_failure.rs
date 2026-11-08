use crate::domain::video::VideoRecordId;

/// Substrings that, found (case-insensitively) in a failure reason, mark a
/// video permanently unavailable — see design.md's token table. This is an
/// allowlist of *permanent* reasons, never an inverted "everything except
/// transient" rule, so an unseen `yt-dlp`/YouTube wording is never silently
/// excluded forever.
const PERMANENT_UNAVAILABILITY_TOKENS: [&str; 8] = [
    "members-only",
    "claimed content",
    "copyright",
    "in your country",
    "private video",
    "has been removed",
    "no longer available",
    "account associated with this video has been terminated",
];

/// Why a download failed cleanly, as the downloader reported it (empty when
/// it reported nothing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadFailure {
    reason: String,
}

impl DownloadFailure {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Whether the reason names a block that can never succeed on a later
    /// attempt, so the video should be excluded rather than retried.
    pub fn is_permanent(&self) -> bool {
        let reason = self.reason.to_lowercase();
        PERMANENT_UNAVAILABILITY_TOKENS
            .iter()
            .any(|token| reason.contains(token))
    }

    /// The reason, or a generic message naming `video_id` when there is none.
    pub fn message_for(&self, video_id: &VideoRecordId) -> String {
        if self.reason.is_empty() {
            format!("yt-dlp failed to download video {video_id}")
        } else {
            self.reason.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_be_permanent_for_every_permanent_unavailability_reason() {
        let reasons = [
            "Join this channel to get access to members-only content like this video, and other exclusive perks.",
            "It was blocked due to the claimed content by Mediatoon.",
            "Video unavailable. This video contains content that has been blocked on copyright grounds.",
            "The uploader has not made this video available in your country.",
            "Private video. Sign in if you've been granted access to this video.",
            "This video has been removed by the uploader.",
            "This video is no longer available.",
            "This video is not available because the YouTube account associated with this video has been terminated.",
        ];

        assert_eq!(
            reasons.map(|reason| DownloadFailure::new(reason).is_permanent()),
            [true; 8]
        );
    }

    #[test]
    fn it_should_be_permanent_regardless_of_case() {
        let reasons = [
            "JOIN THIS CHANNEL FOR MEMBERS-ONLY CONTENT",
            "It was blocked due to the CLAIMED CONTENT by X.",
            "Blocked on Copyright grounds",
            "Not made available IN YOUR COUNTRY",
            "PRIVATE VIDEO",
            "This video Has Been Removed",
            "This video is No Longer Available.",
            "The Account Associated With This Video Has Been Terminated.",
        ];

        assert_eq!(
            reasons.map(|reason| DownloadFailure::new(reason).is_permanent()),
            [true; 8]
        );
    }

    #[test]
    fn it_should_be_permanent_if_a_combined_reason_contains_a_permanent_part() {
        let failure = DownloadFailure::new(
            "This video has been removed by the uploader. | Video unavailable",
        );

        assert!(failure.is_permanent());
    }

    #[test]
    fn it_should_not_be_permanent_for_generic_or_transient_reasons() {
        let reasons = [
            "Video unavailable",
            "HTTP Error 403: Forbidden",
            "Unable to download webpage: The read operation timed out",
            "HTTP Error 429: Too Many Requests",
            "Sign in to confirm you're not a bot",
            "Unable to download video data: fragment 3 not found",
            "Connection reset by peer",
            "",
        ];

        assert_eq!(
            reasons.map(|reason| DownloadFailure::new(reason).is_permanent()),
            [false; 8]
        );
    }

    #[test]
    fn it_should_expose_its_reason() {
        assert_eq!(
            DownloadFailure::new("HTTP Error 403: Forbidden").reason(),
            "HTTP Error 403: Forbidden"
        );
    }

    #[test]
    fn it_should_describe_itself_by_its_reason() {
        let video_id = VideoRecordId::new("v1").unwrap();

        assert_eq!(
            DownloadFailure::new("HTTP Error 403: Forbidden").message_for(&video_id),
            "HTTP Error 403: Forbidden"
        );
    }

    #[test]
    fn it_should_describe_itself_generically_if_it_has_no_reason() {
        let video_id = VideoRecordId::new("v1").unwrap();

        assert_eq!(
            DownloadFailure::new("").message_for(&video_id),
            "yt-dlp failed to download video v1"
        );
    }
}
