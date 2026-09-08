use super::errors::PlaylistIdError;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlaylistId(String);

impl PlaylistId {
    pub fn new(id: impl Into<String>) -> Result<Self, PlaylistIdError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(PlaylistIdError(
                "YouTube playlist ID must not be empty".to_string(),
            ));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_url(&self) -> String {
        format!("https://www.youtube.com/playlist?list={}", self.0)
    }
}

impl fmt::Display for PlaylistId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PlaylistId {
    type Err = PlaylistIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_non_empty_id() {
        let id = PlaylistId::new("PLabc123").unwrap();
        assert_eq!(id.as_str(), "PLabc123");
    }

    #[test]
    fn it_should_reject_an_empty_id() {
        assert!(PlaylistId::new("").is_err());
        assert!(PlaylistId::new("   ").is_err());
    }

    #[test]
    fn it_should_build_the_youtube_playlist_url() {
        let id = PlaylistId::new("PLabc123").unwrap();
        assert_eq!(
            id.to_url(),
            "https://www.youtube.com/playlist?list=PLabc123"
        );
    }
}
