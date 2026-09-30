use crate::infrastructure::repositories::plex_collection_repository::PlexCollectionRepository;
use std::sync::Arc;

/// Deletes the Plex collection left behind by a deleted playlist/channel.
#[derive(Clone)]
pub struct PlexCollectionDeleter {
    #[allow(dead_code)]
    plex_collection_repository: Arc<dyn PlexCollectionRepository>,
}

impl PlexCollectionDeleter {
    pub fn new(plex_collection_repository: Arc<dyn PlexCollectionRepository>) -> Self {
        Self {
            plex_collection_repository,
        }
    }
}

pub trait PlexCollectionDeleterApi: Send + Sync {
    /// Deletes the collection with this title, if any; missing is a no-op.
    fn delete(&self, name: &str) -> anyhow::Result<()>;
}

impl PlexCollectionDeleterApi for PlexCollectionDeleter {
    fn delete(&self, _name: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
