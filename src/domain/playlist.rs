use chrono::{DateTime, Utc};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistError(pub String);

impl fmt::Display for PlaylistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PlaylistError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct YoutubePlaylistId(String);

impl YoutubePlaylistId {
    pub fn new(id: impl Into<String>) -> Result<Self, PlaylistError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(PlaylistError(
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

impl fmt::Display for YoutubePlaylistId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for YoutubePlaylistId {
    type Err = PlaylistError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistName(String);

const FILESYSTEM_UNSAFE_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

impl PlaylistName {
    pub fn new(name: impl Into<String>) -> Result<Self, PlaylistError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(PlaylistError("Playlist name must not be empty".to_string()));
        }
        if name.contains(FILESYSTEM_UNSAFE_CHARS) {
            return Err(PlaylistError(format!(
                "Playlist name must not contain any of these characters: {}",
                FILESYSTEM_UNSAFE_CHARS.iter().collect::<String>()
            )));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlaylistName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    pub id: YoutubePlaylistId,
    pub name: PlaylistName,
    pub created_at: DateTime<Utc>,
}

impl Playlist {
    pub fn create(id: YoutubePlaylistId, name: PlaylistName, created_at: DateTime<Utc>) -> Self {
        Self {
            id,
            name,
            created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_playlist_id_accepts_non_empty_input() {
        let id = YoutubePlaylistId::new("PLabc123").unwrap();
        assert_eq!(id.as_str(), "PLabc123");
    }

    #[test]
    fn youtube_playlist_id_rejects_empty_input() {
        assert!(YoutubePlaylistId::new("").is_err());
        assert!(YoutubePlaylistId::new("   ").is_err());
    }

    #[test]
    fn youtube_playlist_id_builds_url() {
        let id = YoutubePlaylistId::new("PLabc123").unwrap();
        assert_eq!(
            id.to_url(),
            "https://www.youtube.com/playlist?list=PLabc123"
        );
    }

    #[test]
    fn playlist_name_accepts_valid_names() {
        assert!(PlaylistName::new("My Favorite Songs").is_ok());
    }

    #[test]
    fn playlist_name_rejects_empty_names() {
        assert!(PlaylistName::new("").is_err());
        assert!(PlaylistName::new("   ").is_err());
    }

    #[test]
    fn playlist_name_rejects_filesystem_unsafe_characters() {
        assert!(PlaylistName::new("a/b").is_err());
        assert!(PlaylistName::new("a\\b").is_err());
    }

    #[test]
    fn playlist_create_builds_from_valid_value_objects() {
        let id = YoutubePlaylistId::new("PLabc123").unwrap();
        let name = PlaylistName::new("My Playlist").unwrap();
        let created_at = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let playlist = Playlist::create(id.clone(), name.clone(), created_at);

        assert_eq!(playlist.id, id);
        assert_eq!(playlist.name, name);
        assert_eq!(playlist.created_at, created_at);
    }
}
