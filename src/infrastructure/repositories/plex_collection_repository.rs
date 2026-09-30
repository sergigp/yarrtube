use crate::domain::plex::{PlexCollection, PlexItem};
use serde::Deserialize;
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

pub struct PlexConfig {
    pub base_url: String,
    pub token: String,
    pub section_id: String,
}

#[derive(Debug, Deserialize)]
struct MediaContainerResponse {
    #[serde(rename = "MediaContainer")]
    media_container: MediaContainer,
}

#[derive(Debug, Default, Deserialize)]
struct MediaContainer {
    #[serde(rename = "Metadata", default)]
    metadata: Vec<Metadata>,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    #[serde(rename = "ratingKey")]
    rating_key: String,
    #[serde(default)]
    title: String,
    #[serde(rename = "Guid", default)]
    guids: Vec<Guid>,
}

#[derive(Debug, Deserialize)]
struct Guid {
    id: String,
}

impl Metadata {
    /// The YouTube video ID Plex's NFO agent derived from yarrtube's
    /// `<uniqueid type="youtube">`, if this item has one.
    fn youtube_video_id(&self) -> Option<String> {
        self.guids
            .iter()
            .find_map(|guid| guid.id.strip_prefix("youtube://"))
            .map(str::to_string)
    }
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
        let response: MediaContainerResponse = self.get_json(
            &format!("/library/sections/{}/all", self.config.section_id),
            &[("includeGuids", "1")],
        )?;
        Ok(response
            .media_container
            .metadata
            .into_iter()
            .filter_map(|metadata| {
                metadata
                    .youtube_video_id()
                    .map(|youtube_video_id| PlexItem {
                        rating_key: metadata.rating_key,
                        youtube_video_id,
                    })
            })
            .collect())
    }

    fn list_collections(&self) -> anyhow::Result<Vec<PlexCollection>> {
        let response: MediaContainerResponse = self.get_json(
            &format!("/library/sections/{}/collections", self.config.section_id),
            &[],
        )?;
        Ok(response
            .media_container
            .metadata
            .into_iter()
            .map(|metadata| PlexCollection {
                rating_key: metadata.rating_key,
                title: metadata.title,
            })
            .collect())
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

impl HttpPlexCollectionRepository {
    fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> anyhow::Result<T> {
        let response = self
            .client
            .get(format!("{}{path}", self.config.base_url))
            .query(query)
            .header("Accept", "application/json")
            .header("X-Plex-Token", &self.config.token)
            .send()?;
        Self::ensure_success(path, &response)?;
        Ok(response.json()?)
    }

    fn ensure_success(path: &str, response: &reqwest::blocking::Response) -> anyhow::Result<()> {
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("Plex request to {path} failed with status {status}");
        }
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
    /// Every mutating call, in order, e.g. `create:Lofi beats`,
    /// `add:c1:102`, `remove:c1:101`, `delete:c1`.
    mutations: Mutex<Vec<String>>,
    failing_create_titles: Vec<String>,
    unreachable: bool,
}

#[cfg(test)]
impl FakePlexCollectionRepository {
    /// A section whose scanned items are `items`, with no collections yet.
    pub fn with_items(items: Vec<PlexItem>) -> Self {
        Self::with_items_and_collections(items, vec![])
    }

    /// A section whose scanned items are `items` and whose collections
    /// already hold `collections`.
    pub fn with_items_and_collections(
        items: Vec<PlexItem>,
        collections: Vec<FakePlexCollection>,
    ) -> Self {
        Self {
            items,
            collections: Mutex::new(collections),
            mutations: Mutex::new(vec![]),
            failing_create_titles: vec![],
            unreachable: false,
        }
    }

    /// A Plex server that is down: every call fails.
    pub fn failing() -> Self {
        Self {
            unreachable: true,
            ..Self::with_items(vec![])
        }
    }

    /// The same fake, but `create_collection` fails for `title`.
    pub fn failing_create_for(mut self, title: &str) -> Self {
        self.failing_create_titles.push(title.to_string());
        self
    }

    pub fn collections(&self) -> Vec<FakePlexCollection> {
        self.collections.lock().unwrap().clone()
    }

    pub fn mutations(&self) -> Vec<String> {
        self.mutations.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl PlexCollectionRepository for FakePlexCollectionRepository {
    fn list_items(&self) -> anyhow::Result<Vec<PlexItem>> {
        if self.unreachable {
            anyhow::bail!("Plex is unreachable");
        }
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
        if self.failing_create_titles.iter().any(|t| t == title) {
            anyhow::bail!("Plex refused to create the collection {title}");
        }
        self.mutations
            .lock()
            .unwrap()
            .push(format!("create:{title}"));
        self.collections.lock().unwrap().push(FakePlexCollection {
            rating_key: format!("collection:{title}"),
            title: title.to_string(),
            member_rating_keys: rating_keys.to_vec(),
        });
        Ok(())
    }

    fn add_items(&self, collection_rating_key: &str, rating_keys: &[String]) -> anyhow::Result<()> {
        self.mutations.lock().unwrap().extend(
            rating_keys
                .iter()
                .map(|rating_key| format!("add:{collection_rating_key}:{rating_key}")),
        );
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
        self.mutations
            .lock()
            .unwrap()
            .push(format!("remove:{collection_rating_key}:{rating_key}"));
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
        self.mutations
            .lock()
            .unwrap()
            .push(format!("delete:{collection_rating_key}"));
        self.collections
            .lock()
            .unwrap()
            .retain(|collection| collection.rating_key != collection_rating_key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_list_section_items_with_their_youtube_ids() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/library/sections/1/all")
            .match_query(mockito::Matcher::UrlEncoded(
                "includeGuids".into(),
                "1".into(),
            ))
            .match_header("accept", "application/json")
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .with_body(
                r#"{"MediaContainer": {"Metadata": [
                    {"ratingKey": "101", "title": "First video",
                     "Guid": [{"id": "youtube://yt1"}]},
                    {"ratingKey": "102", "title": "Local file without guids"},
                    {"ratingKey": "103", "title": "Non-youtube guid",
                     "Guid": [{"id": "imdb://tt42"}]}
                ]}}"#,
            )
            .create();
        let repository = repository(&server);

        let items = repository.list_items().unwrap();

        assert_eq!(
            items,
            vec![PlexItem {
                rating_key: "101".to_string(),
                youtube_video_id: "yt1".to_string(),
            }]
        );
    }

    #[test]
    fn it_should_list_collections() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/library/sections/1/collections")
            .match_header("accept", "application/json")
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .with_body(
                r#"{"MediaContainer": {"Metadata": [
                    {"ratingKey": "c1", "title": "Lofi beats"},
                    {"ratingKey": "c2", "title": "Some Channel"}
                ]}}"#,
            )
            .create();
        let repository = repository(&server);

        let collections = repository.list_collections().unwrap();

        assert_eq!(
            collections,
            vec![
                PlexCollection {
                    rating_key: "c1".to_string(),
                    title: "Lofi beats".to_string(),
                },
                PlexCollection {
                    rating_key: "c2".to_string(),
                    title: "Some Channel".to_string(),
                },
            ]
        );
    }

    fn repository(server: &mockito::Server) -> HttpPlexCollectionRepository {
        HttpPlexCollectionRepository::new(PlexConfig {
            base_url: server.url(),
            token: "secret-token".to_string(),
            section_id: "1".to_string(),
        })
    }
}
