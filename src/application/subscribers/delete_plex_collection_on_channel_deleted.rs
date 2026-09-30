use crate::domain::services::{PlexCollectionDeleter, PlexCollectionDeleterApi};
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;

/// Reacts to `ChannelDeleted` by deleting the Plex collection named after
/// the deleted channel. The channel row is already gone, so the name
/// travels on the event payload.
pub struct DeletePlexCollectionOnChannelDeleted {
    deleter: PlexCollectionDeleter,
}

impl DeletePlexCollectionOnChannelDeleted {
    pub fn new(deleter: PlexCollectionDeleter) -> Self {
        Self { deleter }
    }
}

impl EventSubscriber for DeletePlexCollectionOnChannelDeleted {
    fn handle(&self, _payload: &str) -> anyhow::Result<()> {
        self.deleter.delete("")
    }
}
