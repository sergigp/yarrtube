use super::playlist::{Playlist, YoutubePlaylistId};
use chrono::{DateTime, Utc};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryError(pub String);

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for RepositoryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupError(pub String);

impl fmt::Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for LookupError {}

/// Outcome of a `PlaylistRepository::save` call. Create is idempotent: saving
/// a playlist whose ID already exists makes no change and returns the
/// pre-existing record rather than erroring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    Created(Playlist),
    AlreadyExisted(Playlist),
}

pub trait PlaylistRepository: Send + Sync {
    fn save(&self, playlist: &Playlist) -> Result<SaveOutcome, RepositoryError>;
    /// Returns `true` if a playlist with this ID was found and deleted, `false` if not found.
    fn delete(&self, id: &YoutubePlaylistId) -> Result<bool, RepositoryError>;
    fn list(&self) -> Result<Vec<Playlist>, RepositoryError>;
}

pub trait YoutubePlaylistLookup: Send + Sync {
    fn exists(&self, id: &YoutubePlaylistId) -> Result<bool, LookupError>;
}

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}
