use crate::domain::category::{Category, CategoryMember, CategoryView, CategoryViews};
use crate::domain::services::{CategoryVideoCollector, CategoryVideoCollectorApi};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use std::collections::HashSet;
use std::sync::Arc;

/// Reads categories as `CategoryViews`.
#[derive(Clone)]
pub struct CategoryViewSearcher {
    repository: Arc<dyn CategoryRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    collector: CategoryVideoCollector,
}

impl CategoryViewSearcher {
    pub fn new(
        repository: Arc<dyn CategoryRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        collector: CategoryVideoCollector,
    ) -> Self {
        Self {
            repository,
            channel_repository,
            playlist_repository,
            collector,
        }
    }
}

pub trait CategoryViewSearcherApi: Send + Sync {
    /// Every category by name ignoring case, with its existing members and
    /// distinct unwatched count, and the distinct total across all.
    fn search_all(&self) -> anyhow::Result<CategoryViews>;
}

impl CategoryViewSearcherApi for CategoryViewSearcher {
    fn search_all(&self) -> anyhow::Result<CategoryViews> {
        let tracked = self.tracked_members()?;
        let unwatched = self.unwatched_by_category()?;
        Ok(CategoryViews {
            unwatched_total: Self::distinct_total(&unwatched),
            categories: unwatched
                .into_iter()
                .map(|(category, ids)| Self::category_view(category, &tracked, ids.len()))
                .collect(),
        })
    }
}

impl CategoryViewSearcher {
    /// Every tracked channel and playlist, as a member.
    fn tracked_members(&self) -> anyhow::Result<HashSet<CategoryMember>> {
        let channels = self
            .channel_repository
            .list()?
            .into_iter()
            .map(|channel| CategoryMember::Channel(channel.id));
        let playlists = self
            .playlist_repository
            .list()?
            .into_iter()
            .map(|playlist| CategoryMember::Playlist(playlist.id));
        Ok(channels.chain(playlists).collect())
    }

    /// Every category paired with its distinct unwatched YouTube IDs.
    fn unwatched_by_category(&self) -> anyhow::Result<Vec<(Category, HashSet<VideoId>)>> {
        self.repository
            .list()?
            .into_iter()
            .map(|category| {
                let ids = self.unwatched_youtube_ids(&category)?;
                Ok((category, ids))
            })
            .collect()
    }

    /// The YouTube IDs of the category's downloaded, unwatched videos, once each.
    fn unwatched_youtube_ids(&self, category: &Category) -> anyhow::Result<HashSet<VideoId>> {
        Ok(self
            .collector
            .downloaded_videos(category)?
            .into_iter()
            .filter(|sourced| sourced.video.is_downloaded_and_unwatched())
            .map(|sourced| sourced.video.youtube_id)
            .collect())
    }

    /// The distinct YouTube IDs across every category's unwatched set.
    fn distinct_total(unwatched: &[(Category, HashSet<VideoId>)]) -> usize {
        unwatched
            .iter()
            .flat_map(|(_, ids)| ids)
            .collect::<HashSet<_>>()
            .len()
    }

    /// The category's view, listing only its `tracked` members.
    fn category_view(
        category: Category,
        tracked: &HashSet<CategoryMember>,
        unwatched_count: usize,
    ) -> CategoryView {
        let members: Vec<&CategoryMember> = category
            .members
            .iter()
            .filter(|member| tracked.contains(member))
            .collect();
        CategoryView {
            channels: members
                .iter()
                .filter_map(|member| match member {
                    CategoryMember::Channel(handle) => Some(handle.clone()),
                    CategoryMember::Playlist(_) => None,
                })
                .collect(),
            playlists: members
                .iter()
                .filter_map(|member| match member {
                    CategoryMember::Playlist(id) => Some(id.clone()),
                    CategoryMember::Channel(_) => None,
                })
                .collect(),
            id: category.id,
            name: category.name,
            unwatched_count,
        }
    }
}
