use crate::domain::plex::{PlexCollection, PlexItem};
use std::sync::Mutex;
use std::time::Duration;

/// The task sharing the serial Light lane must not hold the slot on a hung
/// Plex server, so every request gets an explicit short timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Port to the Plex server's collections HTTP API for the configured
/// library section, authenticated via `X-Plex-Token`.
#[allow(dead_code)] // methods become used as the reconciler/deleter grow
pub trait PlexCollectionRepository: Send + Sync {
    /// Every scanned item in the section carrying a `youtube://` guid.
    fn list_items(&self) -> anyhow::Result<Vec<PlexItem>>;
    fn list_collections(&self) -> anyhow::Result<Vec<PlexCollection>>;
    fn list_collection_items(&self, collection_rating_key: &str) -> anyhow::Result<Vec<PlexItem>>;
    /// Creates the collection with the given members and sets its sort to
    /// alphabetical (POST /library/collections + collectionSort pref).
    fn create_collection(&self, title: &str, rating_keys: &[String]) -> anyhow::Result<()>;
    fn add_items(&self, collection_rating_key: &str, rating_keys: &[String]) -> anyhow::Result<()>;
    fn remove_item(&self, collection_rating_key: &str, rating_key: &str) -> anyhow::Result<()>;
    fn delete_collection(&self, collection_rating_key: &str) -> anyhow::Result<()>;
}

#[allow(dead_code)] // read once the HTTP adapter issues real requests
pub struct PlexConfig {
    pub base_url: String,
    pub token: String,
    pub section_id: String,
}

pub struct HttpPlexCollectionRepository {
    #[allow(dead_code)]
    config: PlexConfig,
    #[allow(dead_code)]
    client: reqwest::blocking::Client,
    /// The server's machine identifier, fetched from `/identity` on first
    /// use and cached for the process lifetime.
    #[allow(dead_code)]
    machine_id: Mutex<Option<String>>,
}

impl HttpPlexCollectionRepository {
    pub fn new(config: PlexConfig) -> Self {
        Self {
            config,
            client: reqwest::blocking::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()
                .expect("failed to build the Plex HTTP client"),
            machine_id: Mutex::new(None),
        }
    }
}

impl PlexCollectionRepository for HttpPlexCollectionRepository {
    fn list_items(&self) -> anyhow::Result<Vec<PlexItem>> {
        Ok(vec![])
    }

    fn list_collections(&self) -> anyhow::Result<Vec<PlexCollection>> {
        Ok(vec![])
    }

    fn list_collection_items(&self, _collection_rating_key: &str) -> anyhow::Result<Vec<PlexItem>> {
        Ok(vec![])
    }

    fn create_collection(&self, _title: &str, _rating_keys: &[String]) -> anyhow::Result<()> {
        Ok(())
    }

    fn add_items(
        &self,
        _collection_rating_key: &str,
        _rating_keys: &[String],
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn remove_item(&self, _collection_rating_key: &str, _rating_key: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn delete_collection(&self, _collection_rating_key: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

/// A collection as the fake stores it: the `PlexCollection` the server
/// would report plus its members' rating keys.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakePlexCollection {
    pub rating_key: String,
    pub title: String,
    pub member_rating_keys: Vec<String>,
}

#[cfg(test)]
#[derive(Default)]
pub struct FakePlexCollectionRepository {
    items: Vec<PlexItem>,
    collections: Mutex<Vec<FakePlexCollection>>,
}

#[cfg(test)]
impl FakePlexCollectionRepository {
    /// A section whose scanned items are `items`, with no collections yet.
    pub fn with_items(items: Vec<PlexItem>) -> Self {
        Self {
            items,
            collections: Mutex::new(vec![]),
        }
    }

    pub fn collections(&self) -> Vec<FakePlexCollection> {
        self.collections.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl PlexCollectionRepository for FakePlexCollectionRepository {
    fn list_items(&self) -> anyhow::Result<Vec<PlexItem>> {
        Ok(self.items.clone())
    }

    fn list_collections(&self) -> anyhow::Result<Vec<PlexCollection>> {
        Ok(self
            .collections
            .lock()
            .unwrap()
            .iter()
            .map(|collection| PlexCollection {
                rating_key: collection.rating_key.clone(),
                title: collection.title.clone(),
            })
            .collect())
    }

    fn list_collection_items(&self, collection_rating_key: &str) -> anyhow::Result<Vec<PlexItem>> {
        Ok(self
            .collections
            .lock()
            .unwrap()
            .iter()
            .filter(|collection| collection.rating_key == collection_rating_key)
            .flat_map(|collection| collection.member_rating_keys.clone())
            .map(|member| PlexItem {
                youtube_video_id: self
                    .items
                    .iter()
                    .find(|item| item.rating_key == member)
                    .map(|item| item.youtube_video_id.clone())
                    .unwrap_or_default(),
                rating_key: member,
            })
            .collect())
    }

    fn create_collection(&self, title: &str, rating_keys: &[String]) -> anyhow::Result<()> {
        self.collections.lock().unwrap().push(FakePlexCollection {
            rating_key: format!("collection:{title}"),
            title: title.to_string(),
            member_rating_keys: rating_keys.to_vec(),
        });
        Ok(())
    }

    fn add_items(&self, collection_rating_key: &str, rating_keys: &[String]) -> anyhow::Result<()> {
        self.collections
            .lock()
            .unwrap()
            .iter_mut()
            .filter(|collection| collection.rating_key == collection_rating_key)
            .for_each(|collection| {
                collection
                    .member_rating_keys
                    .extend(rating_keys.iter().cloned())
            });
        Ok(())
    }

    fn remove_item(&self, collection_rating_key: &str, rating_key: &str) -> anyhow::Result<()> {
        self.collections
            .lock()
            .unwrap()
            .iter_mut()
            .filter(|collection| collection.rating_key == collection_rating_key)
            .for_each(|collection| {
                collection
                    .member_rating_keys
                    .retain(|key| key != rating_key)
            });
        Ok(())
    }

    fn delete_collection(&self, collection_rating_key: &str) -> anyhow::Result<()> {
        self.collections
            .lock()
            .unwrap()
            .retain(|collection| collection.rating_key != collection_rating_key);
        Ok(())
    }
}
