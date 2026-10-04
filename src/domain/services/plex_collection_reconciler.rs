use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::plex::PlexItem;
use crate::domain::video::{VideoRecordId, VideoStatus};
use crate::infrastructure::repositories::plex_collection_repository::PlexCollectionRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::error_report;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Converges every tracked playlist's and channel's Plex collection toward
/// yarrtube's downloaded videos in one pass, matching them by YouTube ID.
/// Each configured section is scoped to exactly one kind: a playlist section
/// is reconciled against tracked playlists only and a channel section against
/// tracked channels only, so a video shared between a playlist and a
/// subscribed channel can never leak a channel collection into a playlist
/// library (or vice versa).
#[derive(Clone)]
pub struct PlexCollectionReconciler {
    playlist_section_ids: Vec<String>,
    channel_section_ids: Vec<String>,
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
        playlist_section_ids: Vec<String>,
        channel_section_ids: Vec<String>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        plex_collection_repository: Arc<dyn PlexCollectionRepository>,
    ) -> Self {
        Self {
            playlist_section_ids,
            channel_section_ids,
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
    /// One convergence pass: each playlist section is reconciled against
    /// tracked playlists only and each channel section against tracked
    /// channels only. Per-collection failures are logged and skipped; a
    /// section-wide failure (e.g. its item listing) is logged, the remaining
    /// sections still reconcile, and the first such error is returned.
    fn reconcile_all(&self) -> anyhow::Result<()>;
}

impl PlexCollectionReconcilerApi for PlexCollectionReconciler {
    fn reconcile_all(&self) -> anyhow::Result<()> {
        let mut first_error = None;
        for section_id in &self.playlist_section_ids {
            if let Err(e) = self.reconcile_playlist_section(section_id) {
                error!(section = section_id, error = %error_report::cause_chain(&e), "failed to reconcile Plex playlist section, skipping");
                first_error.get_or_insert(e);
            }
        }
        for section_id in &self.channel_section_ids {
            if let Err(e) = self.reconcile_channel_section(section_id) {
                error!(section = section_id, error = %error_report::cause_chain(&e), "failed to reconcile Plex channel section, skipping");
                first_error.get_or_insert(e);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

/// The guid prefix of a match Plex's NFO agent builds from a `movie.nfo`
/// whose `uniqueid` is a YouTube video ID.
const NFO_YOUTUBE_GUID_PREFIX: &str = "tv.plex.agents.nfo.movie://movie/youtube_";

/// The section's identified items, keyed by YouTube video ID.
fn identified_by_youtube_id(items: Vec<PlexItem>) -> HashMap<String, String> {
    items
        .into_iter()
        .filter_map(|item| Some((item.youtube_video_id?, item.rating_key)))
        .collect()
}

/// One collection's failure never aborts the pass; the next pass retries it.
fn log_and_skip_failure(name: &str, result: anyhow::Result<()>) {
    if let Err(e) = result {
        error!(collection = name, error = %error_report::cause_chain(&e), "failed to reconcile Plex collection, skipping");
    }
}

impl PlexCollectionReconciler {
    /// Converges one playlist section's collections toward its tracked
    /// playlists, never touching tracked channels.
    fn reconcile_playlist_section(&self, section_id: &str) -> anyhow::Result<()> {
        let (scanned, collections) = self.section_setup(section_id)?;
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
        Ok(())
    }

    /// Converges one channel section's collections toward its tracked
    /// channels, never touching tracked playlists.
    fn reconcile_channel_section(&self, section_id: &str) -> anyhow::Result<()> {
        let (scanned, collections) = self.section_setup(section_id)?;
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

    /// The per-section state both kinds' passes start from: the section's
    /// scanned items keyed by YouTube ID, and its existing collections keyed
    /// by title. Items scanned without a YouTube ID are re-matched first, so
    /// they are keyed by it from the next pass on.
    fn section_setup(
        &self,
        section_id: &str,
    ) -> anyhow::Result<(HashMap<String, String>, HashMap<String, String>)> {
        let items = self.plex_collection_repository.list_items(section_id)?;
        self.rematch_unidentified(section_id, &items);
        let scanned = identified_by_youtube_id(items);
        let collections = self.collections_by_title(section_id)?;
        info!(
            section = section_id,
            scanned_items = scanned.len(),
            collections = collections.len(),
            "reconciling Plex section"
        );
        Ok((scanned, collections))
    }

    /// Plex binds an item's YouTube ID only when it first matches it, so an
    /// item imported before its `movie.nfo` existed stays unidentified until
    /// it is matched again to the candidate the NFO agent builds from that
    /// `movie.nfo`. Never fails the pass: an item without such a candidate,
    /// or whose match fails, is logged and retried next pass.
    fn rematch_unidentified(&self, section_id: &str, items: &[PlexItem]) {
        items
            .iter()
            .filter(|item| item.youtube_video_id.is_none())
            .for_each(|item| match self.rematch(&item.rating_key) {
                Ok(Some(guid)) => {
                    info!(section = section_id, rating_key = %item.rating_key, guid = %guid, "re-matched unidentified Plex item to its movie.nfo")
                }
                Ok(None) => {
                    warn!(section = section_id, rating_key = %item.rating_key, "no YouTube match candidate for unidentified Plex item, skipping")
                }
                Err(e) => {
                    warn!(section = section_id, rating_key = %item.rating_key, error = %error_report::cause_chain(&e), "failed to re-match unidentified Plex item, skipping")
                }
            });
    }

    /// Matches the item to its NFO agent YouTube candidate, returning that
    /// candidate's guid, or `None` when Plex offers no such candidate.
    fn rematch(&self, rating_key: &str) -> anyhow::Result<Option<String>> {
        let candidate = self
            .plex_collection_repository
            .list_match_candidates(rating_key)?
            .into_iter()
            .find(|candidate| candidate.guid.starts_with(NFO_YOUTUBE_GUID_PREFIX));
        candidate
            .map(|candidate| {
                self.plex_collection_repository
                    .match_item(rating_key, &candidate)
                    .map(|()| candidate.guid)
            })
            .transpose()
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
                self.converge_members(section_id, name, collection_rating_key, &desired)
            }
        }
    }

    /// Adds the desired members an existing collection is missing and
    /// removes the members no longer desired. An empty collection is
    /// recreated with the desired members instead, since Plex may stop
    /// accepting additions to it.
    fn converge_members(
        &self,
        section_id: &str,
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
        if members.is_empty() && !desired.is_empty() {
            return self.recreate_collection(section_id, name, collection_rating_key, desired);
        }

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

    /// Deletes an empty collection and creates it again with `desired`,
    /// recovering a collection Plex no longer accepts additions to. It holds
    /// nothing, so nothing is lost.
    fn recreate_collection(
        &self,
        section_id: &str,
        name: &str,
        collection_rating_key: &str,
        desired: &[String],
    ) -> anyhow::Result<()> {
        self.plex_collection_repository
            .delete_collection(collection_rating_key)?;
        self.plex_collection_repository
            .create_collection(section_id, name, desired)?;
        info!(
            section = section_id,
            collection = name,
            members = desired.len(),
            "recreated empty Plex collection"
        );
        Ok(())
    }
}
