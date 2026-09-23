use super::playlist_kind::PlaylistKind;
use super::playlist_name::PlaylistName;
use super::playlist_path::PlaylistPath;
use crate::domain::shared::{PlaylistId, Quality};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: PlaylistName,
    pub path: PlaylistPath,
    pub quality: Quality,
    pub kind: PlaylistKind,
    pub created_at: DateTime<Utc>,
}

impl Playlist {
    pub fn create(
        id: PlaylistId,
        name: PlaylistName,
        path: PlaylistPath,
        quality: Quality,
        kind: PlaylistKind,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            path,
            quality,
            kind,
            created_at,
        }
    }
}
