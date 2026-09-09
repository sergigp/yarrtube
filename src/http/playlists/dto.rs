use crate::domain::playlist::Playlist;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreatePlaylistRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub quality: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PlaylistResponse {
    pub id: String,
    pub name: String,
    pub quality: String,
    pub created_at: DateTime<Utc>,
}

impl From<Playlist> for PlaylistResponse {
    fn from(playlist: Playlist) -> Self {
        Self {
            id: playlist.id.as_str().to_string(),
            name: playlist.name.as_str().to_string(),
            quality: playlist.quality.as_str().to_string(),
            created_at: playlist.created_at,
        }
    }
}
