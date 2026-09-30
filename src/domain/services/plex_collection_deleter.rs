use crate::infrastructure::repositories::plex_collection_repository::PlexCollectionRepository;
use std::sync::Arc;
use tracing::info;

/// Deletes the Plex collection left behind by a deleted playlist/channel.
#[derive(Clone)]
pub struct PlexCollectionDeleter {
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
    fn delete(&self, name: &str) -> anyhow::Result<()> {
        let collection = self
            .plex_collection_repository
            .list_collections()?
            .into_iter()
            .find(|collection| collection.title == name);

        match collection {
            Some(collection) => {
                self.plex_collection_repository
                    .delete_collection(&collection.rating_key)?;
                info!(collection = name, "deleted Plex collection");
                Ok(())
            }
            None => Ok(()),
        }
    }
}
