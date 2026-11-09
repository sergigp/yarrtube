use crate::domain::shared::ValidationError;
use std::fmt;
use uuid::Uuid;

/// A stored video's own identifier. Distinct from its `YoutubeVideoId`: the
/// same YouTube video is stored once per playlist or channel holding it, each
/// copy with its own `VideoId`. Generated once when a `Video` is first
/// created, so a domain service can reference it (e.g. from an owning
/// `PlaylistVideo`/`ChannelVideo` row) before that `Video` is persisted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VideoId(String);

impl VideoId {
    /// Parses an ID as stored (or as supplied by a caller that
    /// already has one, e.g. a task payload).
    pub fn new(id: impl Into<String>) -> Result<Self, ValidationError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(ValidationError("Video ID must not be empty".to_string()));
        }
        Ok(Self(id))
    }

    /// Generates a new, unique ID for a freshly created `Video`.
    pub fn new_generated() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VideoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_generate_distinct_ids() {
        assert_ne!(VideoId::new_generated(), VideoId::new_generated());
    }

    #[test]
    fn it_should_accept_a_non_empty_id() {
        assert_eq!(VideoId::new("rec1"), Ok(VideoId("rec1".to_string())));
    }

    #[test]
    fn it_should_reject_an_empty_id() {
        assert_eq!(
            VideoId::new(""),
            Err(ValidationError("Video ID must not be empty".to_string()))
        );
    }

    #[test]
    fn it_should_reject_a_blank_id() {
        assert_eq!(
            VideoId::new("   "),
            Err(ValidationError("Video ID must not be empty".to_string()))
        );
    }
}
