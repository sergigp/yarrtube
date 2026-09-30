use crate::infrastructure::repositories::plex_collection_repository::PlexCollectionRepository;
use std::sync::Arc;
use tracing::info;

/// Deletes the Plex collections left behind by a deleted playlist/channel,
/// across every configured library section.
#[derive(Clone)]
pub struct PlexCollectionDeleter {
    section_ids: Vec<String>,
    plex_collection_repository: Arc<dyn PlexCollectionRepository>,
}

impl PlexCollectionDeleter {
    pub fn new(
        section_ids: Vec<String>,
        plex_collection_repository: Arc<dyn PlexCollectionRepository>,
    ) -> Self {
        Self {
            section_ids,
            plex_collection_repository,
        }
    }
}

pub trait PlexCollectionDeleterApi: Send + Sync {
    /// Deletes the collection with this title from every configured
    /// section where one exists; missing everywhere is a no-op.
    fn delete(&self, name: &str) -> anyhow::Result<()>;
}

impl PlexCollectionDeleterApi for PlexCollectionDeleter {
    fn delete(&self, name: &str) -> anyhow::Result<()> {
        for section_id in &self.section_ids {
            let collection = self
                .plex_collection_repository
                .list_collections(section_id)?
                .into_iter()
                .find(|collection| collection.title == name);

            if let Some(collection) = collection {
                self.plex_collection_repository
                    .delete_collection(&collection.rating_key)?;
                info!(
                    section = section_id,
                    collection = name,
                    "deleted Plex collection"
                );
            }
        }
        Ok(())
    }
}
