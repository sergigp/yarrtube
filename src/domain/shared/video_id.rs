use super::errors::VideoIdError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VideoId(String);

impl VideoId {
    pub fn new(id: impl Into<String>) -> Result<Self, VideoIdError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(VideoIdError(
                "YouTube video ID must not be empty".to_string(),
            ));
        }
        Ok(Self(id))
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
    fn it_should_accept_a_non_empty_id() {
        let id = VideoId::new("vid1").unwrap();
        assert_eq!(id.as_str(), "vid1");
    }

    #[test]
    fn it_should_reject_an_empty_id() {
        assert!(VideoId::new("").is_err());
        assert!(VideoId::new("   ").is_err());
    }
}
