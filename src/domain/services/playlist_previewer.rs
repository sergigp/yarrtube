use crate::domain::playlist::{PlaylistId, PlaylistName, PlaylistPreview, PreviewPlaylistError};
use crate::infrastructure::repositories::youtube_playlist_repository::YoutubePlaylistRepository;
use std::sync::Arc;

/// Looks a YouTube playlist up without tracking it.
#[derive(Clone)]
pub struct PlaylistPreviewer {
    lookup: Arc<dyn YoutubePlaylistRepository>,
}

impl PlaylistPreviewer {
    pub fn new(lookup: Arc<dyn YoutubePlaylistRepository>) -> Self {
        Self { lookup }
    }
}

pub trait PlaylistPreviewerApi: Send + Sync {
    /// Returns the playlist's ID, YouTube title and video count, persisting
    /// nothing.
    fn preview(&self, id: PlaylistId) -> Result<PlaylistPreview, PreviewPlaylistError>;
}

impl PlaylistPreviewerApi for PlaylistPreviewer {
    fn preview(&self, id: PlaylistId) -> Result<PlaylistPreview, PreviewPlaylistError> {
        match self.lookup.resolve(&id) {
            Ok(Some(resolved)) => Ok(PlaylistPreview {
                name: PlaylistName::from_youtube_title(&resolved.title, &id),
                video_count: resolved.item_count,
                id,
            }),
            _ => Err(PreviewPlaylistError::YoutubePlaylistNotFound(id)),
        }
    }
}
