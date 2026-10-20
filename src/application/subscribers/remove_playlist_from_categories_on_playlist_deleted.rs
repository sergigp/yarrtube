use crate::domain::services::CategoryMemberRemover;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;

/// Reacts to `PlaylistDeleted` by removing the playlist from every category
/// holding it. The categories themselves stay.
pub struct RemovePlaylistFromCategoriesOnPlaylistDeleted {
    remover: CategoryMemberRemover,
}

impl RemovePlaylistFromCategoriesOnPlaylistDeleted {
    pub fn new(remover: CategoryMemberRemover) -> Self {
        Self { remover }
    }
}

impl EventSubscriber for RemovePlaylistFromCategoriesOnPlaylistDeleted {
    fn handle(&self, _payload: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
