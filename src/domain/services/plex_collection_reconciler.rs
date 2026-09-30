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
use tracing::{debug, error, info};

/// Converges every tracked playlist's and channel's Plex collection toward
/// yarrtube's downloaded videos in one pass, matching them by YouTube ID.
/// Covers every configured library section: a collection materializes in
/// the section(s) whose library holds that playlist's/channel's scanned
/// videos.
#[derive(Clone)]
pub struct PlexCollectionReconciler {
    section_ids: Vec<String>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    plex_collection_repository: Arc<dyn PlexCollectionRepository>,
}

impl PlexCollectionReconciler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        section_ids: Vec<String>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        plex_collection_repository: Arc<dyn PlexCollectionRepository>,
    ) -> Self {
        Self {
            section_ids,
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
    /// One convergence pass over every configured section, each covering
    /// every playlist and channel. Per-collection failures are logged and
    /// skipped; a section-wide failure (e.g. its item listing) is logged,
    /// the remaining sections still reconcile, and the first such error is
    /// returned.
    fn reconcile_all(&self) -> anyhow::Result<()>;
}

impl PlexCollectionReconcilerApi for PlexCollectionReconciler {
    fn reconcile_all(&self) -> anyhow::Result<()> {
        let mut first_error = None;
        for section_id in &self.section_ids {
            if let Err(e) = self.reconcile_section(section_id) {
                error!(section = section_id, error = %e, "failed to reconcile Plex section, skipping");
                first_error.get_or_insert(e);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

/// One collection's failure never aborts the pass; the next pass retries it.
fn log_and_skip_failure(name: &str, result: anyhow::Result<()>) {
    if let Err(e) = result {
        error!(collection = name, error = %e, "failed to reconcile Plex collection, skipping");
    }
}

impl PlexCollectionReconciler {
    /// Converges one section's collections toward yarrtube's state.
    fn reconcile_section(&self, section_id: &str) -> anyhow::Result<()> {
        let scanned = self.scanned_items_by_youtube_id(section_id)?;
        let collections = self.collections_by_title(section_id)?;
        info!(
            section = section_id,
            scanned_items = scanned.len(),
            collections = collections.len(),
            "reconciling Plex section"
        );

        for playlist in self.playlist_repository.list()? {
            let result = self
                .playlist_desired_rating_keys(&playlist.id, &scanned)
                .and_then(|desired| {
                    self.reconcile_collection(
                        section_id,
                        playlist.name.as_str(),
                        desired,
                        &collections,
                    )
                });
            log_and_skip_failure(playlist.name.as_str(), result);
        }
        for channel in self.channel_repository.list()? {
            let result = self
                .channel_desired_rating_keys(&channel.id, &scanned)
                .and_then(|desired| {
                    self.reconcile_collection(section_id, &channel.name, desired, &collections)
                });
            log_and_skip_failure(&channel.name, result);
        }
        Ok(())
    }

    /// The section's scanned items, keyed by YouTube video ID.
    fn scanned_items_by_youtube_id(
        &self,
        section_id: &str,
    ) -> anyhow::Result<HashMap<String, String>> {
        Ok(self
            .plex_collection_repository
            .list_items(section_id)?
            .into_iter()
            .map(|item| (item.youtube_video_id, item.rating_key))
            .collect())
    }

    fn collections_by_title(&self, section_id: &str) -> anyhow::Result<HashMap<String, String>> {
        Ok(self
            .plex_collection_repository
            .list_collections(section_id)?
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
        section_id: &str,
        name: &str,
        desired: Vec<String>,
        collections: &HashMap<String, String>,
    ) -> anyhow::Result<()> {
        match collections.get(name) {
            None if desired.is_empty() => {
                debug!(
                    section = section_id,
                    collection = name,
                    "no scanned videos in this section, not creating the collection"
                );
                Ok(())
            }
            None => {
                self.plex_collection_repository
                    .create_collection(section_id, name, &desired)?;
                info!(
                    section = section_id,
                    collection = name,
                    members = desired.len(),
                    "created Plex collection"
                );
                Ok(())
            }
            Some(collection_rating_key) => {
                self.converge_members(name, collection_rating_key, &desired)
            }
        }
    }

    /// Adds the desired members an existing collection is missing and
    /// removes the members no longer desired.
    fn converge_members(
        &self,
        name: &str,
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
            info!(
                collection = name,
                added = missing.len(),
                "added items to Plex collection"
            );
        }

        for member in members.iter().filter(|member| !desired.contains(member)) {
            self.plex_collection_repository
                .remove_item(collection_rating_key, member)?;
            info!(
                collection = name,
                rating_key = member,
                "removed item from Plex collection"
            );
        }
        Ok(())
    }
}
