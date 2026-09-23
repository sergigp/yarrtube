use super::errors::ValidationError;
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
    pub fn new(id: impl Into<String>) -> Result<Self, ValidationError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(ValidationError(
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
        assert_ne!(
            VideoRecordId::new_generated(),
            VideoRecordId::new_generated()
        );
    }

    #[test]
    fn it_should_accept_a_non_empty_id() {
        assert_eq!(
            VideoRecordId::new("rec1"),
            Ok(VideoRecordId("rec1".to_string()))
        );
    }

    #[test]
    fn it_should_reject_an_empty_id() {
        assert_eq!(
            VideoRecordId::new(""),
            Err(ValidationError(
                "Video record ID must not be empty".to_string()
            ))
        );
    }

    #[test]
    fn it_should_reject_a_blank_id() {
        assert_eq!(
            VideoRecordId::new("   "),
            Err(ValidationError(
                "Video record ID must not be empty".to_string()
            ))
        );
    }
}
