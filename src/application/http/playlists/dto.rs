use crate::domain::playlist::{Playlist, PlaylistPreview, PlaylistView};
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
    #[serde(default)]
    pub reconcile_interval_minutes: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlaylistRequest {
    #[serde(default)]
    pub exclude_from_home: Option<bool>,
    #[serde(default)]
    pub hide_unwatched_count: Option<bool>,
    #[serde(default)]
    pub reconcile_interval_minutes: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct MovePlaylistRequest {
    #[serde(default)]
    pub path: Option<String>,
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
    pub hide_unwatched_count: bool,
    pub reconcile_interval_minutes: u32,
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
            hide_unwatched_count: playlist.hide_unwatched_count,
            reconcile_interval_minutes: playlist.reconcile_interval.minutes(),
            created_at: playlist.created_at,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PlaylistListItemResponse {
    pub id: String,
    pub name: String,
    pub path: String,
    pub quality: String,
    pub kind: String,
    pub exclude_from_home: bool,
    pub hide_unwatched_count: bool,
    pub reconcile_interval_minutes: u32,
    pub created_at: DateTime<Utc>,
    pub unwatched_count: usize,
    pub work_in_progress: bool,
}

impl From<PlaylistView> for PlaylistListItemResponse {
    fn from(playlist: PlaylistView) -> Self {
        Self {
            id: playlist.id.as_str().to_string(),
            name: playlist.name.as_str().to_string(),
            path: playlist.path.as_str().to_string(),
            quality: playlist.quality.as_str().to_string(),
            kind: playlist.kind.as_str().to_string(),
            exclude_from_home: playlist.exclude_from_home,
            hide_unwatched_count: playlist.hide_unwatched_count,
            reconcile_interval_minutes: playlist.reconcile_interval.minutes(),
            created_at: playlist.created_at,
            unwatched_count: playlist.unwatched_count,
            work_in_progress: playlist.work_in_progress,
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
