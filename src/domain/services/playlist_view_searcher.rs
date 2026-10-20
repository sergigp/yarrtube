use crate::domain::playlist::{Playlist, PlaylistView};
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::sync::Arc;

/// Reads playlists as `PlaylistView`s.
#[derive(Clone)]
pub struct PlaylistViewSearcher {
    repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
}

impl PlaylistViewSearcher {
    pub fn new(
        repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
    ) -> Self {
        Self {
            repository,
            playlist_video_repository,
            video_repository,
        }
    }
}

pub trait PlaylistViewSearcherApi: Send + Sync {
    fn search_all(&self) -> anyhow::Result<Vec<PlaylistView>>;
}

impl PlaylistViewSearcherApi for PlaylistViewSearcher {
    fn search_all(&self) -> anyhow::Result<Vec<PlaylistView>> {
        Ok(self
            .repository
            .list()?
            .into_iter()
            .map(Self::playlist_view)
            .collect())
    }
}

impl PlaylistViewSearcher {
    fn playlist_view(playlist: Playlist) -> PlaylistView {
        PlaylistView {
            id: playlist.id,
            name: playlist.name,
            path: playlist.path,
            quality: playlist.quality,
            kind: playlist.kind,
            exclude_from_home: playlist.exclude_from_home,
            hide_unwatched_count: playlist.hide_unwatched_count,
            reconcile_interval: playlist.reconcile_interval,
            created_at: playlist.created_at,
            unwatched_count: 0,
        }
    }
}
