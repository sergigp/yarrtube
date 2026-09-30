use crate::infrastructure::repositories::plex_collection_repository::PlexCollectionRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::sync::Arc;

/// Converges every tracked playlist's and channel's Plex collection toward
/// yarrtube's downloaded videos in one pass, matching them by YouTube ID.
#[derive(Clone)]
pub struct PlexCollectionReconciler {
    #[allow(dead_code)]
    playlist_repository: Arc<dyn PlaylistRepository>,
    #[allow(dead_code)]
    channel_repository: Arc<dyn ChannelRepository>,
    #[allow(dead_code)]
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    #[allow(dead_code)]
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    #[allow(dead_code)]
    video_repository: Arc<dyn VideoRepository>,
    #[allow(dead_code)]
    plex_collection_repository: Arc<dyn PlexCollectionRepository>,
}

impl PlexCollectionReconciler {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        plex_collection_repository: Arc<dyn PlexCollectionRepository>,
    ) -> Self {
        Self {
            playlist_repository,
            channel_repository,
            playlist_video_repository,
            channel_video_repository,
            video_repository,
            plex_collection_repository,
        }
    }
}

pub trait PlexCollectionReconcilerApi: Send + Sync {
    /// One convergence pass over every playlist and channel. Per-collection
    /// failures are logged and skipped; only pass-wide failures (e.g. the
    /// section listing) return Err.
    fn reconcile_all(&self) -> anyhow::Result<()>;
}

impl PlexCollectionReconcilerApi for PlexCollectionReconciler {
    fn reconcile_all(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
