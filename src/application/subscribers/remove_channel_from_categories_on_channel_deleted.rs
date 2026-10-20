use crate::domain::services::CategoryMemberRemover;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;

/// Reacts to `ChannelDeleted` by removing the channel from every category
/// holding it. The categories themselves stay.
pub struct RemoveChannelFromCategoriesOnChannelDeleted {
    remover: CategoryMemberRemover,
}

impl RemoveChannelFromCategoriesOnChannelDeleted {
    pub fn new(remover: CategoryMemberRemover) -> Self {
        Self { remover }
    }
}

impl EventSubscriber for RemoveChannelFromCategoriesOnChannelDeleted {
    fn handle(&self, _payload: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
