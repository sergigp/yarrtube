use crate::domain::category::{Category, CategoryMember};
use crate::domain::video::{SourcedVideo, Video, VideoRecordId, VideoSource};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_categories_repository::VideoCategoriesRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::collections::HashSet;
use std::sync::Arc;

/// Collects the downloaded videos of a category's members, shared by the
/// paths that list, count and mark a category's videos.
#[derive(Clone)]
pub struct CategoryVideoCollector {
    channel_repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_categories_repository: Arc<dyn VideoCategoriesRepository>,
}

impl CategoryVideoCollector {
    pub fn new(
        channel_repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        video_categories_repository: Arc<dyn VideoCategoriesRepository>,
    ) -> Self {
        Self {
            channel_repository,
            channel_video_repository,
            playlist_repository,
            playlist_video_repository,
            video_repository,
            video_categories_repository,
        }
    }
}

pub trait CategoryVideoCollectorApi: Send + Sync {
    /// Downloaded videos belonging to the category that still exist, each
    /// with its playlist or channel: channel sources first, then playlist
    /// sources, each in the category's member order, sources no longer
    /// members last. Sources already deleted are skipped.
    fn downloaded_videos(&self, category: &Category) -> anyhow::Result<Vec<SourcedVideo>>;
}

impl CategoryVideoCollectorApi for CategoryVideoCollector {
    fn downloaded_videos(&self, category: &Category) -> anyhow::Result<Vec<SourcedVideo>> {
        let video_ids = self.video_ids_in(category)?;
        let mut groups = self.records_by_source(&video_ids)?;
        groups.sort_by_key(|(source, _)| Self::source_rank(category, source));
        Ok(groups
            .iter()
            .map(|(source, ids)| self.downloaded_from(source, ids))
            .collect::<anyhow::Result<Vec<_>>>()?
            .concat())
    }
}

impl CategoryVideoCollector {
    fn video_ids_in(&self, category: &Category) -> anyhow::Result<HashSet<VideoRecordId>> {
        Ok(self
            .video_categories_repository
            .list_in_category(&category.id)?
            .into_iter()
            .map(|categories| categories.video_id)
            .collect())
    }

    /// The records of `video_ids` grouped by their source, sources and
    /// records in storage order.
    fn records_by_source(
        &self,
        video_ids: &HashSet<VideoRecordId>,
    ) -> anyhow::Result<Vec<(CategoryMember, Vec<VideoRecordId>)>> {
        if video_ids.is_empty() {
            return Ok(Vec::new());
        }
        let channel_records =
            self.channel_video_repository
                .list()?
                .into_iter()
                .map(|channel_video| {
                    (
                        CategoryMember::Channel(channel_video.channel_id),
                        channel_video.video_id,
                    )
                });
        let playlist_records =
            self.playlist_video_repository
                .list()?
                .into_iter()
                .map(|playlist_video| {
                    (
                        CategoryMember::Playlist(playlist_video.playlist_id),
                        playlist_video.video_id,
                    )
                });
        Ok(channel_records
            .chain(playlist_records)
            .filter(|(_, video_id)| video_ids.contains(video_id))
            .fold(
                Vec::<(CategoryMember, Vec<VideoRecordId>)>::new(),
                |mut grouped, (source, video_id)| {
                    match grouped.iter_mut().find(|(kept, _)| *kept == source) {
                        Some((_, ids)) => ids.push(video_id),
                        None => grouped.push((source, vec![video_id])),
                    }
                    grouped
                },
            ))
    }

    /// The downloaded videos of `video_ids`, from `source`; none when the
    /// source is gone.
    fn downloaded_from(
        &self,
        source: &CategoryMember,
        video_ids: &[VideoRecordId],
    ) -> anyhow::Result<Vec<SourcedVideo>> {
        match self.video_source(source)? {
            Some(source) => self.downloaded(video_ids, source),
            None => Ok(Vec::new()),
        }
    }

    fn video_source(&self, source: &CategoryMember) -> anyhow::Result<Option<VideoSource>> {
        Ok(match source {
            CategoryMember::Channel(handle) => {
                self.channel_repository
                    .find(handle)?
                    .map(|channel| VideoSource::Channel {
                        handle: channel.id,
                        name: channel.name,
                        path: channel.path,
                        avatar_filename: channel.avatar_filename,
                    })
            }
            CategoryMember::Playlist(id) => {
                self.playlist_repository
                    .find(id)?
                    .map(|playlist| VideoSource::Playlist {
                        id: playlist.id,
                        name: playlist.name,
                        path: playlist.path,
                    })
            }
        })
    }

    /// The downloaded videos of `video_ids`, each from `source`.
    fn downloaded(
        &self,
        video_ids: &[VideoRecordId],
        source: VideoSource,
    ) -> anyhow::Result<Vec<SourcedVideo>> {
        Ok(self
            .video_repository
            .find_many(video_ids)?
            .into_iter()
            .filter(Video::is_downloaded)
            .map(|video| SourcedVideo {
                video,
                source: source.clone(),
            })
            .collect())
    }

    /// Members before former members, channels before playlists, then
    /// member order.
    fn source_rank(category: &Category, source: &CategoryMember) -> (bool, bool, usize) {
        let position = category.members.iter().position(|member| member == source);
        (
            position.is_none(),
            matches!(source, CategoryMember::Playlist(_)),
            position.unwrap_or(usize::MAX),
        )
    }
}
