use crate::domain::event::DomainEvent;
use crate::domain::playlist::PlaylistKind;
use crate::domain::shared::{PlaylistId, VideoId};
use crate::domain::video::{RemoveVideoFromPlaylistError, VideoStatus};
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use std::sync::Arc;
use tracing::info;

/// Removes a video from a custom playlist, hard-deleting its stored record
/// and publishing `VideoRemovedFromPlaylist` for the existing cleanup path.
#[derive(Clone)]
pub struct CustomPlaylistVideoRemover {
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_repository: Arc<dyn VideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl CustomPlaylistVideoRemover {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_repository: Arc<dyn VideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            playlist_repository,
            video_repository,
            playlist_video_repository,
            event_publisher,
        }
    }

    pub fn remove(
        &self,
        playlist_id: PlaylistId,
        video_id: VideoId,
    ) -> Result<(), RemoveVideoFromPlaylistError> {
        let playlist = self
            .playlist_repository
            .find(&playlist_id)
            .map_err(RemoveVideoFromPlaylistError::Repository)?
            .ok_or_else(|| RemoveVideoFromPlaylistError::PlaylistNotFound(playlist_id.clone()))?;
        if playlist.kind != PlaylistKind::Custom {
            return Err(RemoveVideoFromPlaylistError::NotCustomPlaylist(playlist_id));
        }

        let playlist_video = self
            .playlist_video_repository
            .find_by_youtube_video(&playlist_id, &video_id)
            .map_err(RemoveVideoFromPlaylistError::Repository)?
            .ok_or_else(|| RemoveVideoFromPlaylistError::VideoNotFound(video_id.clone()))?;
        let video = self
            .video_repository
            .find(&playlist_video.video_id)
            .map_err(RemoveVideoFromPlaylistError::Repository)?
            .ok_or_else(|| RemoveVideoFromPlaylistError::VideoNotFound(video_id.clone()))?;

        self.playlist_video_repository
            .delete(&playlist_id, &video_id)
            .map_err(RemoveVideoFromPlaylistError::Repository)?;
        self.video_repository
            .delete(&video.id)
            .map_err(RemoveVideoFromPlaylistError::Repository)?;
        self.event_publisher
            .publish(&DomainEvent::VideoRemovedFromPlaylist {
                playlist_id: playlist_id.as_str().to_string(),
                video_id: video.id.as_str().to_string(),
                title: video.title.clone(),
                filename: video.filename.clone(),
                thumbnail_filename: video.thumbnail_filename.clone(),
                was_downloaded: video.status == VideoStatus::Downloaded,
            })
            .map_err(RemoveVideoFromPlaylistError::Repository)?;
        info!(playlist_id = %playlist_id, video_id = %video_id, "removed video from custom playlist");
        Ok(())
    }
}
