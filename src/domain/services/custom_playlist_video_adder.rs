use crate::domain::event::DomainEvent;
use crate::domain::playlist::PlaylistKind;
use crate::domain::playlist_video::PlaylistVideo;
use crate::domain::services::thumbnail_fetcher::ThumbnailFetcher;
use crate::domain::shared::{PlaylistId, VideoId};
use crate::domain::video::{AddVideoToCustomPlaylistError, Video, resolve_output_dir};
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_video_repository::YoutubeVideoRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;
use tracing::info;

/// Adds a video to a custom playlist, confirming it exists and is
/// accessible on YouTube (and fetching its title from there) before
/// persisting.
#[derive(Clone)]
pub struct CustomPlaylistVideoAdder {
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_repository: Arc<dyn VideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    youtube_video_repository: Arc<dyn YoutubeVideoRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    thumbnail_fetcher: Arc<ThumbnailFetcher>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl CustomPlaylistVideoAdder {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_repository: Arc<dyn VideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        youtube_video_repository: Arc<dyn YoutubeVideoRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        thumbnail_fetcher: Arc<ThumbnailFetcher>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            playlist_repository,
            video_repository,
            playlist_video_repository,
            youtube_video_repository,
            event_publisher,
            thumbnail_fetcher,
            clock,
            videos_path: videos_path.into(),
        }
    }

    /// No-ops if the video is already stored for this playlist.
    pub fn add(
        &self,
        playlist_id: PlaylistId,
        video_id: VideoId,
    ) -> Result<(), AddVideoToCustomPlaylistError> {
        let playlist = self
            .playlist_repository
            .find(&playlist_id)
            .map_err(AddVideoToCustomPlaylistError::Repository)?
            .ok_or_else(|| AddVideoToCustomPlaylistError::PlaylistNotFound(playlist_id.clone()))?;
        if playlist.kind != PlaylistKind::Custom {
            return Err(AddVideoToCustomPlaylistError::NotCustomPlaylist(
                playlist_id,
            ));
        }

        if self
            .playlist_video_repository
            .find_by_youtube_video(&playlist_id, &video_id)
            .map_err(AddVideoToCustomPlaylistError::Repository)?
            .is_some()
        {
            return Ok(());
        }

        let youtube_video = self
            .youtube_video_repository
            .find(&video_id)
            .map_err(AddVideoToCustomPlaylistError::Lookup)?
            .ok_or_else(|| AddVideoToCustomPlaylistError::YoutubeVideoNotFound(video_id.clone()))?;

        let now = self.clock.now();
        let video = Video::create(video_id.clone(), youtube_video.title, now);
        self.video_repository
            .save(&video)
            .map_err(AddVideoToCustomPlaylistError::Repository)?;
        let playlist_video = PlaylistVideo::create(playlist_id.clone(), video.id.clone(), now);
        self.playlist_video_repository
            .save(&playlist_video)
            .map_err(AddVideoToCustomPlaylistError::Repository)?;
        let output_dir = resolve_output_dir(&self.videos_path, playlist.path.as_str());
        self.thumbnail_fetcher.fetch(&video, &output_dir);
        self.event_publisher
            .publish(&DomainEvent::VideoAddedToPlaylist {
                playlist_id: playlist_id.as_str().to_string(),
                video_id: video.id.as_str().to_string(),
            })
            .map_err(AddVideoToCustomPlaylistError::Repository)?;
        info!(playlist_id = %playlist_id, video_id = %video_id, "added video to custom playlist");
        Ok(())
    }
}
