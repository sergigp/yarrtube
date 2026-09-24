use crate::domain::event::DomainEvent;
use crate::domain::playlist::errors::DeletePlaylistError;
use crate::domain::shared::PlaylistId;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use std::sync::Arc;
use tracing::info;

/// Deletes a playlist and every video row stored under it.
#[derive(Clone)]
pub struct PlaylistDeleter {
    repository: Arc<dyn PlaylistRepository>,
    video_repository: Arc<dyn VideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl PlaylistDeleter {
    pub fn new(
        repository: Arc<dyn PlaylistRepository>,
        video_repository: Arc<dyn VideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            repository,
            video_repository,
            playlist_video_repository,
            event_publisher,
        }
    }
}

pub trait PlaylistDeleterApi: Send + Sync {
    fn delete(&self, id: PlaylistId) -> Result<(), DeletePlaylistError>;
}

impl PlaylistDeleterApi for PlaylistDeleter {
    fn delete(&self, id: PlaylistId) -> Result<(), DeletePlaylistError> {
        let playlist = match self.repository.find(&id) {
            Ok(Some(playlist)) => playlist,
            Ok(None) => return Err(DeletePlaylistError::NotFound(id)),
            Err(e) => return Err(DeletePlaylistError::Repository(e)),
        };

        let playlist_videos = self
            .playlist_video_repository
            .list_for_playlist(&id)
            .map_err(DeletePlaylistError::Repository)?;
        for playlist_video in &playlist_videos {
            self.video_repository
                .delete(&playlist_video.video_id)
                .map_err(DeletePlaylistError::Repository)?;
        }
        self.playlist_video_repository
            .delete_all_for_playlist(&id)
            .map_err(DeletePlaylistError::Repository)?;

        self.repository
            .delete(&id)
            .map_err(DeletePlaylistError::Repository)?;
        self.event_publisher
            .publish(&DomainEvent::PlaylistDeleted {
                playlist_id: id.as_str().to_string(),
                path: playlist.path.as_str().to_string(),
            })
            .map_err(DeletePlaylistError::Repository)?;
        info!(playlist_id = %id, "deleted playlist");
        Ok(())
    }
}
