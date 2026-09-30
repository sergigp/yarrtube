use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::video::{VideoRecordId, VideoStatus};
use crate::infrastructure::repositories::plex_collection_repository::PlexCollectionRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::collections::HashMap;
use std::sync::Arc;

/// Converges every tracked playlist's and channel's Plex collection toward
/// yarrtube's downloaded videos in one pass, matching them by YouTube ID.
#[derive(Clone)]
pub struct PlexCollectionReconciler {
    playlist_repository: Arc<dyn PlaylistRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
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
        let scanned = self.scanned_items_by_youtube_id()?;
        let collections = self.collections_by_title()?;

        for playlist in self.playlist_repository.list()? {
            let desired = self.playlist_desired_rating_keys(&playlist.id, &scanned)?;
            self.reconcile_collection(playlist.name.as_str(), desired, &collections)?;
        }
        for channel in self.channel_repository.list()? {
            let desired = self.channel_desired_rating_keys(&channel.id, &scanned)?;
            self.reconcile_collection(&channel.name, desired, &collections)?;
        }
        Ok(())
    }
}

impl PlexCollectionReconciler {
    /// The section's scanned items, keyed by YouTube video ID.
    fn scanned_items_by_youtube_id(&self) -> anyhow::Result<HashMap<String, String>> {
        Ok(self
            .plex_collection_repository
            .list_items()?
            .into_iter()
            .map(|item| (item.youtube_video_id, item.rating_key))
            .collect())
    }

    fn collections_by_title(&self) -> anyhow::Result<HashMap<String, String>> {
        Ok(self
            .plex_collection_repository
            .list_collections()?
            .into_iter()
            .map(|collection| (collection.title, collection.rating_key))
            .collect())
    }

    /// The rating keys of the playlist's downloaded videos that Plex has
    /// scanned, in playlist order.
    fn playlist_desired_rating_keys(
        &self,
        playlist_id: &PlaylistId,
        scanned: &HashMap<String, String>,
    ) -> anyhow::Result<Vec<String>> {
        let video_ids: Vec<VideoRecordId> = self
            .playlist_video_repository
            .list_for_playlist(playlist_id)?
            .into_iter()
            .map(|playlist_video| playlist_video.video_id)
            .collect();
        self.downloaded_scanned_rating_keys(&video_ids, scanned)
    }

    /// The rating keys of the channel's downloaded videos that Plex has
    /// scanned, most recent first.
    fn channel_desired_rating_keys(
        &self,
        channel_id: &ChannelHandle,
        scanned: &HashMap<String, String>,
    ) -> anyhow::Result<Vec<String>> {
        let video_ids: Vec<VideoRecordId> = self
            .channel_video_repository
            .list_for_channel(channel_id)?
            .into_iter()
            .map(|channel_video| channel_video.video_id)
            .collect();
        self.downloaded_scanned_rating_keys(&video_ids, scanned)
    }

    /// The rating keys of the videos among `video_ids` that are downloaded
    /// and scanned by Plex, in the order given.
    fn downloaded_scanned_rating_keys(
        &self,
        video_ids: &[VideoRecordId],
        scanned: &HashMap<String, String>,
    ) -> anyhow::Result<Vec<String>> {
        Ok(self
            .video_repository
            .find_many(video_ids)?
            .into_iter()
            .filter(|video| video.status == VideoStatus::Downloaded)
            .filter_map(|video| scanned.get(video.youtube_id.as_str()).cloned())
            .collect())
    }

    /// Converges one playlist's/channel's collection: creates it when
    /// missing and there is something to put in it, or brings an existing
    /// one's membership up to date.
    fn reconcile_collection(
        &self,
        name: &str,
        desired: Vec<String>,
        collections: &HashMap<String, String>,
    ) -> anyhow::Result<()> {
        match collections.get(name) {
            None if desired.is_empty() => Ok(()),
            None => self
                .plex_collection_repository
                .create_collection(name, &desired),
            Some(collection_rating_key) => self.converge_members(collection_rating_key, &desired),
        }
    }

    /// Adds the desired members an existing collection is missing and
    /// removes the members no longer desired.
    fn converge_members(
        &self,
        collection_rating_key: &str,
        desired: &[String],
    ) -> anyhow::Result<()> {
        let members: Vec<String> = self
            .plex_collection_repository
            .list_collection_items(collection_rating_key)?
            .into_iter()
            .map(|item| item.rating_key)
            .collect();

        let missing: Vec<String> = desired
            .iter()
            .filter(|rating_key| !members.contains(rating_key))
            .cloned()
            .collect();
        if !missing.is_empty() {
            self.plex_collection_repository
                .add_items(collection_rating_key, &missing)?;
        }

        for member in members.iter().filter(|member| !desired.contains(member)) {
            self.plex_collection_repository
                .remove_item(collection_rating_key, member)?;
        }
        Ok(())
    }
}
