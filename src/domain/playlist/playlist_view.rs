use super::playlist_id::PlaylistId;
use super::playlist_kind::PlaylistKind;
use super::playlist_name::PlaylistName;
use super::playlist_path::PlaylistPath;
use crate::domain::shared::{Quality, ReconcileInterval};
use chrono::{DateTime, Utc};

/// A playlist as listed, with its count of downloaded videos not yet watched
/// and whether it has work in progress (so can't be moved yet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistView {
    pub id: PlaylistId,
    pub name: PlaylistName,
    pub path: PlaylistPath,
    pub quality: Quality,
    pub kind: PlaylistKind,
    pub exclude_from_home: bool,
    pub hide_unwatched_count: bool,
    pub reconcile_interval: ReconcileInterval,
    pub created_at: DateTime<Utc>,
    pub unwatched_count: usize,
    pub work_in_progress: bool,
}
