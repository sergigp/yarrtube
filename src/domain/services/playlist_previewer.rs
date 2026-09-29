use crate::domain::playlist::{PlaylistId, PlaylistName, PlaylistPreview, PreviewPlaylistError};
use crate::infrastructure::repositories::youtube_playlist_repository::{
    ResolvedPlaylist, YoutubePlaylistRepository,
};
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
        let resolved = self.resolve_on_youtube(&id)?;
        Ok(PlaylistPreview {
            name: PlaylistName::from_youtube_title(&resolved.title, &id),
            video_count: resolved.item_count,
            id,
        })
    }
}

impl PlaylistPreviewer {
    fn resolve_on_youtube(
        &self,
        id: &PlaylistId,
    ) -> Result<ResolvedPlaylist, PreviewPlaylistError> {
        match self.lookup.resolve(id) {
            Ok(Some(resolved)) => Ok(resolved),
            Ok(None) => Err(PreviewPlaylistError::YoutubePlaylistNotFound(id.clone())),
            Err(e) => Err(PreviewPlaylistError::Lookup(e)),
        }
    }
}
