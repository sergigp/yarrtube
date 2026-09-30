use crate::domain::services::{PlexCollectionDeleter, PlexCollectionDeleterApi};
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;

/// Reacts to `PlaylistDeleted` by deleting the Plex collection named after
/// the deleted playlist. The playlist row is already gone, so the name
/// travels on the event payload.
pub struct DeletePlexCollectionOnPlaylistDeleted {
    deleter: PlexCollectionDeleter,
}

impl DeletePlexCollectionOnPlaylistDeleted {
    pub fn new(deleter: PlexCollectionDeleter) -> Self {
        Self { deleter }
    }
}

impl EventSubscriber for DeletePlexCollectionOnPlaylistDeleted {
    fn handle(&self, _payload: &str) -> anyhow::Result<()> {
        self.deleter.delete("")
    }
}
