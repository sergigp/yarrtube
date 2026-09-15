use super::errors::VideoRecordIdError;
use std::fmt;
use uuid::Uuid;

/// A video's own surrogate identifier, distinct from `VideoId` (the YouTube
/// video ID). Generated once when a `Video` row is first created, so a
/// domain service can reference it (e.g. from an owning `PlaylistVideo`/
/// `ChannelVideo` row) before that `Video` is persisted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VideoRecordId(String);

impl VideoRecordId {
    /// Generates a new, unique surrogate ID for a freshly created `Video`.
    pub fn new_generated() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Parses a surrogate ID as stored (or as supplied by a caller that
    /// already has one, e.g. a task payload).
    pub fn new(id: impl Into<String>) -> Result<Self, VideoRecordIdError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(VideoRecordIdError(
                "Video record ID must not be empty".to_string(),
            ));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VideoRecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_generate_distinct_ids() {
        let a = VideoRecordId::new_generated();
        let b = VideoRecordId::new_generated();

        assert_ne!(a, b);
    }

    #[test]
    fn it_should_accept_a_non_empty_id() {
        let id = VideoRecordId::new("rec1").unwrap();
        assert_eq!(id.as_str(), "rec1");
    }

    #[test]
    fn it_should_reject_an_empty_id() {
        assert!(VideoRecordId::new("").is_err());
        assert!(VideoRecordId::new("   ").is_err());
    }
}
