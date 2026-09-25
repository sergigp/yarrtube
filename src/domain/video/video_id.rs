use crate::domain::shared::ValidationError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VideoId(String);

impl VideoId {
    pub fn new(id: impl Into<String>) -> Result<Self, ValidationError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(ValidationError(
                "YouTube video ID must not be empty".to_string(),
            ));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_url(&self) -> String {
        format!("https://www.youtube.com/watch?v={}", self.0)
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
    fn it_should_accept_a_non_empty_id() {
        assert_eq!(VideoId::new("vid1"), Ok(VideoId("vid1".to_string())));
    }

    #[test]
    fn it_should_reject_an_empty_id() {
        assert_eq!(
            VideoId::new(""),
            Err(ValidationError(
                "YouTube video ID must not be empty".to_string()
            ))
        );
    }

    #[test]
    fn it_should_reject_a_blank_id() {
        assert_eq!(
            VideoId::new("   "),
            Err(ValidationError(
                "YouTube video ID must not be empty".to_string()
            ))
        );
    }

    #[test]
    fn it_should_build_the_youtube_video_url() {
        assert_eq!(
            VideoId("vid1".to_string()).to_url(),
            "https://www.youtube.com/watch?v=vid1"
        );
    }
}
