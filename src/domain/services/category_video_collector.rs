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

/// Collects the downloaded videos belonging to a category (read from the
/// per-video category relation), shared by the paths that list, count and
/// mark a category's videos.
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
        let from_members = self.member_records(category, &video_ids)?;
        let from_former_members = self.former_member_records(&video_ids, &from_members)?;
        Ok(from_members
            .iter()
            .chain(&from_former_members)
            .map(|(source, ids)| self.downloaded_from(source, ids))
            .collect::<anyhow::Result<Vec<_>>>()?
            .concat())
    }
}

impl CategoryVideoCollector {
    /// The records holding the category, in record id order.
    fn video_ids_in(&self, category: &Category) -> anyhow::Result<Vec<VideoRecordId>> {
        Ok(self
            .video_categories_repository
            .list_in_category(&category.id)?
            .into_iter()
            .map(|categories| categories.video_id)
            .collect())
    }

    /// The records of `video_ids` held by each of the category's members,
    /// channels first then playlists, each in member order, records in their
    /// source's order. Members holding none are left out.
    fn member_records(
        &self,
        category: &Category,
        video_ids: &[VideoRecordId],
    ) -> anyhow::Result<Vec<(CategoryMember, Vec<VideoRecordId>)>> {
        if video_ids.is_empty() {
            return Ok(Vec::new());
        }
        let wanted: HashSet<&VideoRecordId> = video_ids.iter().collect();
        let mut members = category.members.clone();
        members.sort_by_key(|member| matches!(member, CategoryMember::Playlist(_)));
        members
            .into_iter()
            .map(|member| {
                let ids = self
                    .record_ids_of(&member)?
                    .into_iter()
                    .filter(|id| wanted.contains(id))
                    .collect::<Vec<_>>();
                Ok((member, ids))
            })
            .filter(|group| !matches!(group, Ok((_, ids)) if ids.is_empty()))
            .collect()
    }

    /// The records of `video_ids` not held by a member (their source left the
    /// category), grouped by source, channels first then playlists.
    fn former_member_records(
        &self,
        video_ids: &[VideoRecordId],
        from_members: &[(CategoryMember, Vec<VideoRecordId>)],
    ) -> anyhow::Result<Vec<(CategoryMember, Vec<VideoRecordId>)>> {
        let grouped: HashSet<&VideoRecordId> =
            from_members.iter().flat_map(|(_, ids)| ids).collect();
        let mut groups = video_ids
            .iter()
            .filter(|id| !grouped.contains(id))
            .map(|id| Ok(self.source_of(id)?.map(|source| (source, id.clone()))))
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .fold(
                Vec::<(CategoryMember, Vec<VideoRecordId>)>::new(),
                |mut groups, (source, id)| {
                    match groups.iter_mut().find(|(kept, _)| *kept == source) {
                        Some((_, ids)) => ids.push(id),
                        None => groups.push((source, vec![id])),
                    }
                    groups
                },
            );
        groups.sort_by_key(|(source, _)| matches!(source, CategoryMember::Playlist(_)));
        Ok(groups)
    }

    /// Every record of `source`, in its source's order.
    fn record_ids_of(&self, source: &CategoryMember) -> anyhow::Result<Vec<VideoRecordId>> {
        Ok(match source {
            CategoryMember::Channel(handle) => self
                .channel_video_repository
                .list_for_channel(handle)?
                .into_iter()
                .map(|channel_video| channel_video.video_id)
                .collect(),
            CategoryMember::Playlist(id) => self
                .playlist_video_repository
                .list_for_playlist(id)?
                .into_iter()
                .map(|playlist_video| playlist_video.video_id)
                .collect(),
        })
    }

    /// The channel holding `video_id`, else the playlist holding it.
    fn source_of(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<CategoryMember>> {
        if let Some(channel_video) = self.channel_video_repository.find_by_video(video_id)? {
            return Ok(Some(CategoryMember::Channel(channel_video.channel_id)));
        }
        Ok(self
            .playlist_video_repository
            .find_by_video(video_id)?
            .map(|playlist_video| CategoryMember::Playlist(playlist_video.playlist_id)))
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
}
