use super::category_id::CategoryId;
use super::category_name::CategoryName;
use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;

/// A category as listed: its existing members and its distinct downloaded,
/// unwatched videos count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryView {
    pub id: CategoryId,
    pub name: CategoryName,
    pub unwatched_count: usize,
    /// Existing members only.
    pub channels: Vec<ChannelHandle>,
    pub playlists: Vec<PlaylistId>,
}

/// Every category, with the distinct unwatched videos across all of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryViews {
    pub categories: Vec<CategoryView>,
    pub unwatched_total: usize,
}
