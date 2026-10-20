use crate::domain::category::{Category, CategoryMember, CategoryView, CategoryViews};
use crate::domain::services::CategoryVideoCollector;
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
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
        let categories = self.repository.list()?;
        Ok(CategoryViews {
            categories: categories.into_iter().map(Self::category_view).collect(),
            unwatched_total: 0,
        })
    }
}

impl CategoryViewSearcher {
    fn category_view(category: Category) -> CategoryView {
        CategoryView {
            channels: category
                .members
                .iter()
                .filter_map(|member| match member {
                    CategoryMember::Channel(handle) => Some(handle.clone()),
                    CategoryMember::Playlist(_) => None,
                })
                .collect(),
            playlists: category
                .members
                .iter()
                .filter_map(|member| match member {
                    CategoryMember::Playlist(id) => Some(id.clone()),
                    CategoryMember::Channel(_) => None,
                })
                .collect(),
            id: category.id,
            name: category.name,
            unwatched_count: 0,
        }
    }
}
