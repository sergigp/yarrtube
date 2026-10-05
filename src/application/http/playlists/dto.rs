use crate::domain::playlist::{Playlist, PlaylistPreview};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreatePlaylistRequest {
    pub playlist: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub exclude_from_home: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlaylistRequest {
    #[serde(default)]
    pub exclude_from_home: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PreviewPlaylistQuery {
    #[serde(default)]
    pub playlist: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PlaylistResponse {
    pub id: String,
    pub name: String,
    pub path: String,
    pub quality: String,
    pub kind: String,
    pub exclude_from_home: bool,
    pub created_at: DateTime<Utc>,
}

impl From<Playlist> for PlaylistResponse {
    fn from(playlist: Playlist) -> Self {
        Self {
            id: playlist.id.as_str().to_string(),
            name: playlist.name.as_str().to_string(),
            path: playlist.path.as_str().to_string(),
            quality: playlist.quality.as_str().to_string(),
            kind: playlist.kind.as_str().to_string(),
            exclude_from_home: playlist.exclude_from_home,
            created_at: playlist.created_at,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PlaylistPreviewResponse {
    pub id: String,
    pub title: String,
    pub video_count: u64,
}

impl From<PlaylistPreview> for PlaylistPreviewResponse {
    fn from(preview: PlaylistPreview) -> Self {
        Self {
            id: preview.id.as_str().to_string(),
            title: preview.name.as_str().to_string(),
            video_count: preview.video_count,
        }
    }
}
