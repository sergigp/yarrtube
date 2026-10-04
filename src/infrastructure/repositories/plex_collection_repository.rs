use crate::domain::plex::{PlexCollection, PlexItem, PlexMatchCandidate, PlexSection};
use serde::Deserialize;
use std::sync::Mutex;
use std::time::Duration;

/// The task sharing the serial Light lane must not hold the slot on a hung
/// Plex server, so every request gets an explicit short timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Port to the Plex server's collections HTTP API, authenticated via
/// `X-Plex-Token`. Reads and creates are scoped to a library section per
/// call (yarrtube-fed content can span several Plex libraries); operations
/// on a rating key take none, since rating keys are server-global in Plex.
pub trait PlexCollectionRepository: Send + Sync {
    /// Every scanned item in the section carrying a `youtube://` guid.
    fn list_items(&self, section_id: &str) -> anyhow::Result<Vec<PlexItem>>;
    fn list_collections(&self, section_id: &str) -> anyhow::Result<Vec<PlexCollection>>;
    fn list_collection_items(&self, collection_rating_key: &str) -> anyhow::Result<Vec<PlexItem>>;
    /// Creates the collection with the given members and sets its sort to
    /// alphabetical (POST /library/collections + collectionSort pref).
    fn create_collection(
        &self,
        section_id: &str,
        title: &str,
        rating_keys: &[String],
    ) -> anyhow::Result<()>;
    fn add_items(&self, collection_rating_key: &str, rating_keys: &[String]) -> anyhow::Result<()>;
    fn remove_item(&self, collection_rating_key: &str, rating_key: &str) -> anyhow::Result<()>;
    fn delete_collection(&self, collection_rating_key: &str) -> anyhow::Result<()>;
    /// Every library section on the server, with the folders it scans.
    fn list_sections(&self) -> anyhow::Result<Vec<PlexSection>>;
    /// Asks Plex to scan only `path` (a server-side folder) in the section,
    /// so a new video is imported without waiting for a library scan.
    fn scan_path(&self, section_id: &str, path: &str) -> anyhow::Result<()>;
    /// The metadata matches Plex offers for the item (its "Fix Match" list).
    fn list_match_candidates(&self, rating_key: &str) -> anyhow::Result<Vec<PlexMatchCandidate>>;
    /// Matches the item to `candidate`, so Plex rebinds its metadata and
    /// guids from it.
    fn match_item(&self, rating_key: &str, candidate: &PlexMatchCandidate) -> anyhow::Result<()>;
}

pub struct PlexConfig {
    pub base_url: String,
    pub token: String,
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
    #[serde(rename = "machineIdentifier")]
    machine_identifier: Option<String>,
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

#[derive(Debug, Deserialize)]
struct SectionsResponse {
    #[serde(rename = "MediaContainer")]
    media_container: SectionsContainer,
}

#[derive(Debug, Deserialize)]
struct SectionsContainer {
    #[serde(rename = "Directory", default)]
    directories: Vec<Directory>,
}

#[derive(Debug, Deserialize)]
struct Directory {
    key: String,
    #[serde(rename = "Location", default)]
    locations: Vec<Location>,
}

#[derive(Debug, Deserialize)]
struct Location {
    path: String,
}

#[derive(Debug, Deserialize)]
struct MatchesResponse {
    #[serde(rename = "MediaContainer")]
    media_container: MatchesContainer,
}

#[derive(Debug, Deserialize)]
struct MatchesContainer {
    #[serde(rename = "SearchResult", default)]
    search_results: Vec<SearchResult>,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    guid: String,
    #[serde(default)]
    name: String,
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
    config: PlexConfig,
    client: reqwest::blocking::Client,
    /// The server's machine identifier, fetched from `/identity` on first
    /// use and cached for the process lifetime.
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
    fn list_items(&self, section_id: &str) -> anyhow::Result<Vec<PlexItem>> {
        self.get_items(&format!("/library/sections/{section_id}/all"))
    }

    fn list_collections(&self, section_id: &str) -> anyhow::Result<Vec<PlexCollection>> {
        let response: MediaContainerResponse =
            self.get_json(&format!("/library/sections/{section_id}/collections"), &[])?;
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

    fn list_collection_items(&self, collection_rating_key: &str) -> anyhow::Result<Vec<PlexItem>> {
        Ok(self
            .get_items(&format!(
                "/library/collections/{collection_rating_key}/children"
            ))?
            .into_iter()
            .filter(|item| item.youtube_video_id.is_some())
            .collect())
    }

    fn create_collection(
        &self,
        section_id: &str,
        title: &str,
        rating_keys: &[String],
    ) -> anyhow::Result<()> {
        let uri = self.members_uri(rating_keys)?;

        let path = "/library/collections";
        let response = self
            .client
            .post(format!("{}{path}", self.config.base_url))
            .query(&[
                ("type", "1"),
                ("smart", "0"),
                ("sectionId", section_id),
                ("title", title),
                ("uri", &uri),
            ])
            .header("Accept", "application/json")
            .header("X-Plex-Token", &self.config.token)
            .send()?;
        let created: MediaContainerResponse = Self::ensure_success(path, response)?.json()?;

        let collection_rating_key = created
            .media_container
            .metadata
            .into_iter()
            .next()
            .map(|metadata| metadata.rating_key)
            .ok_or_else(|| anyhow::anyhow!("Plex create collection response held no metadata"))?;
        self.set_alphabetical_sort(&collection_rating_key)
    }

    fn add_items(&self, collection_rating_key: &str, rating_keys: &[String]) -> anyhow::Result<()> {
        let uri = self.members_uri(rating_keys)?;

        let path = format!("/library/collections/{collection_rating_key}/items");
        let response = self
            .client
            .put(format!("{}{path}", self.config.base_url))
            .query(&[("uri", &uri)])
            .header("Accept", "application/json")
            .header("X-Plex-Token", &self.config.token)
            .send()?;
        Self::ensure_success(&path, response).map(drop)
    }

    fn remove_item(&self, collection_rating_key: &str, rating_key: &str) -> anyhow::Result<()> {
        self.delete(&format!(
            "/library/collections/{collection_rating_key}/items/{rating_key}"
        ))
    }

    fn delete_collection(&self, collection_rating_key: &str) -> anyhow::Result<()> {
        self.delete(&format!("/library/collections/{collection_rating_key}"))
    }

    fn list_sections(&self) -> anyhow::Result<Vec<PlexSection>> {
        let response: SectionsResponse = self.get_json("/library/sections", &[])?;
        Ok(response
            .media_container
            .directories
            .into_iter()
            .map(|directory| PlexSection {
                id: directory.key,
                locations: directory
                    .locations
                    .into_iter()
                    .map(|location| location.path)
                    .collect(),
            })
            .collect())
    }

    fn scan_path(&self, section_id: &str, path: &str) -> anyhow::Result<()> {
        let endpoint = format!("/library/sections/{section_id}/refresh");
        let response = self
            .client
            .get(format!("{}{endpoint}", self.config.base_url))
            .query(&[("path", path)])
            .header("Accept", "application/json")
            .header("X-Plex-Token", &self.config.token)
            .send()?;
        Self::ensure_success(&endpoint, response).map(drop)
    }

    fn list_match_candidates(&self, rating_key: &str) -> anyhow::Result<Vec<PlexMatchCandidate>> {
        let response: MatchesResponse =
            self.get_json(&format!("/library/metadata/{rating_key}/matches"), &[])?;
        Ok(response
            .media_container
            .search_results
            .into_iter()
            .map(|result| PlexMatchCandidate {
                guid: result.guid,
                name: result.name,
            })
            .collect())
    }

    fn match_item(&self, rating_key: &str, candidate: &PlexMatchCandidate) -> anyhow::Result<()> {
        let path = format!("/library/metadata/{rating_key}/match");
        let response = self
            .client
            .put(format!("{}{path}", self.config.base_url))
            .query(&[
                ("guid", candidate.guid.as_str()),
                ("name", candidate.name.as_str()),
            ])
            .header("Accept", "application/json")
            .header("X-Plex-Token", &self.config.token)
            .send()?;
        Self::ensure_success(&path, response).map(drop)
    }
}

impl HttpPlexCollectionRepository {
    /// The server's machine identifier, from `/identity`, fetched once and
    /// cached for the process lifetime.
    fn machine_id(&self) -> anyhow::Result<String> {
        let mut machine_id = self.machine_id.lock().unwrap();
        if let Some(machine_id) = machine_id.as_ref() {
            return Ok(machine_id.clone());
        }

        let response: MediaContainerResponse = self.get_json("/identity", &[])?;
        let fetched = response
            .media_container
            .machine_identifier
            .ok_or_else(|| anyhow::anyhow!("Plex /identity response held no machineIdentifier"))?;
        *machine_id = Some(fetched.clone());
        Ok(fetched)
    }

    fn delete(&self, path: &str) -> anyhow::Result<()> {
        let response = self
            .client
            .delete(format!("{}{path}", self.config.base_url))
            .header("Accept", "application/json")
            .header("X-Plex-Token", &self.config.token)
            .send()?;
        Self::ensure_success(path, response).map(drop)
    }

    /// The `uri` value Plex's collection endpoints take to identify a set
    /// of items on this server.
    fn members_uri(&self, rating_keys: &[String]) -> anyhow::Result<String> {
        let machine_id = self.machine_id()?;
        Ok(format!(
            "server://{machine_id}/com.plexapp.plugins.library/library/metadata/{}",
            rating_keys.join(",")
        ))
    }

    /// Sets the collection to sort alphabetically, so yarrtube's
    /// position-prefixed `sorttitle` values order members by playlist
    /// position / publish date.
    fn set_alphabetical_sort(&self, collection_rating_key: &str) -> anyhow::Result<()> {
        let path = format!("/library/metadata/{collection_rating_key}/prefs");
        let response = self
            .client
            .put(format!("{}{path}", self.config.base_url))
            .query(&[("collectionSort", "1")])
            .header("Accept", "application/json")
            .header("X-Plex-Token", &self.config.token)
            .send()?;
        Self::ensure_success(&path, response).map(drop)
    }

    /// Fetches a metadata listing with each item's YouTube ID, if its
    /// guids carry one, requesting guids explicitly (`includeGuids=1`).
    fn get_items(&self, path: &str) -> anyhow::Result<Vec<PlexItem>> {
        let response: MediaContainerResponse = self.get_json(path, &[("includeGuids", "1")])?;
        Ok(response
            .media_container
            .metadata
            .into_iter()
            .map(|metadata| PlexItem {
                youtube_video_id: metadata.youtube_video_id(),
                rating_key: metadata.rating_key,
            })
            .collect())
    }

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
        Ok(Self::ensure_success(path, response)?.json()?)
    }

    /// Passes a successful response through; otherwise fails with the
    /// status and Plex's (trimmed) response body, if it sent one.
    fn ensure_success(
        path: &str,
        response: reqwest::blocking::Response,
    ) -> anyhow::Result<reqwest::blocking::Response> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let body = response.text().unwrap_or_default();
        match body.trim() {
            "" => anyhow::bail!("Plex request to {path} failed with status {status}"),
            body => anyhow::bail!("Plex request to {path} failed with status {status}: {body}"),
        }
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

/// One library section as the fake stores it: its scanned items and its
/// collections.
#[cfg(test)]
#[derive(Debug, Default, Clone)]
pub struct FakePlexSection {
    pub items: Vec<PlexItem>,
    pub collections: Vec<FakePlexCollection>,
    pub locations: Vec<String>,
}

#[cfg(test)]
#[derive(Default)]
pub struct FakePlexCollectionRepository {
    sections: Mutex<std::collections::BTreeMap<String, FakePlexSection>>,
    /// Every mutating call, in order, e.g. `create:1:Lofi beats`,
    /// `add:c1:102`, `remove:c1:101`, `delete:c1`.
    mutations: Mutex<Vec<String>>,
    failing_create_titles: Vec<String>,
    unreachable: bool,
    /// The match candidates Plex offers per item rating key.
    match_candidates: std::collections::BTreeMap<String, Vec<PlexMatchCandidate>>,
}

#[cfg(test)]
impl FakePlexCollectionRepository {
    /// A single section whose scanned items are `items`, with no
    /// collections yet.
    pub fn with_items(section_id: &str, items: Vec<PlexItem>) -> Self {
        Self::with_items_and_collections(section_id, items, vec![])
    }

    /// A single section whose scanned items are `items` and whose
    /// collections already hold `collections`.
    pub fn with_items_and_collections(
        section_id: &str,
        items: Vec<PlexItem>,
        collections: Vec<FakePlexCollection>,
    ) -> Self {
        Self::default().and_section(section_id, items, collections)
    }

    /// The same fake with another section added.
    pub fn and_section(
        self,
        section_id: &str,
        items: Vec<PlexItem>,
        collections: Vec<FakePlexCollection>,
    ) -> Self {
        self.sections.lock().unwrap().insert(
            section_id.to_string(),
            FakePlexSection {
                items,
                collections,
                locations: vec![],
            },
        );
        self
    }

    /// The same fake with `section_id` scanning the server-side folder
    /// `location` (the section is added if missing).
    pub fn with_location(self, section_id: &str, location: &str) -> Self {
        self.sections
            .lock()
            .unwrap()
            .entry(section_id.to_string())
            .or_default()
            .locations
            .push(location.to_string());
        self
    }

    /// The same fake offering `candidates` as `rating_key`'s matches.
    pub fn with_match_candidates(
        mut self,
        rating_key: &str,
        candidates: Vec<PlexMatchCandidate>,
    ) -> Self {
        self.match_candidates
            .insert(rating_key.to_string(), candidates);
        self
    }

    /// Every section's scanned items, ordered by section id.
    pub fn items(&self) -> Vec<PlexItem> {
        self.sections
            .lock()
            .unwrap()
            .values()
            .flat_map(|section| section.items.clone())
            .collect()
    }

    /// A Plex server that is down: the pass's first call fails.
    pub fn failing() -> Self {
        Self {
            unreachable: true,
            ..Self::default()
        }
    }

    /// The same fake, but `create_collection` fails for `title`.
    pub fn failing_create_for(mut self, title: &str) -> Self {
        self.failing_create_titles.push(title.to_string());
        self
    }

    /// Every section's collections, ordered by section id.
    pub fn collections(&self) -> Vec<FakePlexCollection> {
        self.sections
            .lock()
            .unwrap()
            .values()
            .flat_map(|section| section.collections.clone())
            .collect()
    }

    pub fn collections_in(&self, section_id: &str) -> Vec<FakePlexCollection> {
        self.sections
            .lock()
            .unwrap()
            .get(section_id)
            .map(|section| section.collections.clone())
            .unwrap_or_default()
    }

    pub fn mutations(&self) -> Vec<String> {
        self.mutations.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl PlexCollectionRepository for FakePlexCollectionRepository {
    fn list_items(&self, section_id: &str) -> anyhow::Result<Vec<PlexItem>> {
        if self.unreachable {
            anyhow::bail!("Plex is unreachable");
        }
        Ok(self
            .sections
            .lock()
            .unwrap()
            .get(section_id)
            .map(|section| section.items.clone())
            .unwrap_or_default())
    }

    fn list_collections(&self, section_id: &str) -> anyhow::Result<Vec<PlexCollection>> {
        Ok(self
            .sections
            .lock()
            .unwrap()
            .get(section_id)
            .map(|section| section.collections.clone())
            .unwrap_or_default()
            .into_iter()
            .map(|collection| PlexCollection {
                rating_key: collection.rating_key,
                title: collection.title,
            })
            .collect())
    }

    fn list_collection_items(&self, collection_rating_key: &str) -> anyhow::Result<Vec<PlexItem>> {
        let sections = self.sections.lock().unwrap();
        let all_items: Vec<PlexItem> = sections
            .values()
            .flat_map(|section| section.items.clone())
            .collect();
        Ok(sections
            .values()
            .flat_map(|section| section.collections.iter())
            .filter(|collection| collection.rating_key == collection_rating_key)
            .flat_map(|collection| collection.member_rating_keys.clone())
            .map(|member| PlexItem {
                youtube_video_id: all_items
                    .iter()
                    .find(|item| item.rating_key == member)
                    .and_then(|item| item.youtube_video_id.clone()),
                rating_key: member,
            })
            .collect())
    }

    fn create_collection(
        &self,
        section_id: &str,
        title: &str,
        rating_keys: &[String],
    ) -> anyhow::Result<()> {
        if self.failing_create_titles.iter().any(|t| t == title) {
            anyhow::bail!("Plex refused to create the collection {title}");
        }
        self.mutations
            .lock()
            .unwrap()
            .push(format!("create:{section_id}:{title}"));
        self.sections
            .lock()
            .unwrap()
            .entry(section_id.to_string())
            .or_default()
            .collections
            .push(FakePlexCollection {
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
        self.sections
            .lock()
            .unwrap()
            .values_mut()
            .flat_map(|section| section.collections.iter_mut())
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
        self.sections
            .lock()
            .unwrap()
            .values_mut()
            .flat_map(|section| section.collections.iter_mut())
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
        self.sections
            .lock()
            .unwrap()
            .values_mut()
            .for_each(|section| {
                section
                    .collections
                    .retain(|collection| collection.rating_key != collection_rating_key)
            });
        Ok(())
    }

    fn list_sections(&self) -> anyhow::Result<Vec<PlexSection>> {
        if self.unreachable {
            anyhow::bail!("Plex is unreachable");
        }
        Ok(self
            .sections
            .lock()
            .unwrap()
            .iter()
            .map(|(id, section)| PlexSection {
                id: id.clone(),
                locations: section.locations.clone(),
            })
            .collect())
    }

    fn scan_path(&self, section_id: &str, path: &str) -> anyhow::Result<()> {
        if self.unreachable {
            anyhow::bail!("Plex is unreachable");
        }
        self.mutations
            .lock()
            .unwrap()
            .push(format!("scan:{section_id}:{path}"));
        Ok(())
    }

    fn list_match_candidates(&self, rating_key: &str) -> anyhow::Result<Vec<PlexMatchCandidate>> {
        Ok(self
            .match_candidates
            .get(rating_key)
            .cloned()
            .unwrap_or_default())
    }

    /// Like Plex re-reading the item's `movie.nfo`: matching to a
    /// `…youtube_<id>` candidate gives the item that YouTube ID.
    fn match_item(&self, rating_key: &str, candidate: &PlexMatchCandidate) -> anyhow::Result<()> {
        self.mutations
            .lock()
            .unwrap()
            .push(format!("match:{rating_key}:{}", candidate.guid));
        let youtube_video_id = candidate
            .guid
            .split_once("youtube_")
            .map(|(_, id)| id.to_string());
        self.sections
            .lock()
            .unwrap()
            .values_mut()
            .flat_map(|section| section.items.iter_mut())
            .filter(|item| item.rating_key == rating_key)
            .for_each(|item| item.youtube_video_id = youtube_video_id.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_list_every_section_item_with_its_youtube_id_if_any() {
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

        let items = repository.list_items("1").unwrap();

        assert_eq!(
            items,
            vec![
                PlexItem {
                    rating_key: "101".to_string(),
                    youtube_video_id: Some("yt1".to_string()),
                },
                PlexItem {
                    rating_key: "102".to_string(),
                    youtube_video_id: None,
                },
                PlexItem {
                    rating_key: "103".to_string(),
                    youtube_video_id: None,
                },
            ]
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

        let collections = repository.list_collections("1").unwrap();

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

    #[test]
    fn it_should_list_collection_items() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/library/collections/c1/children")
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
                    {"ratingKey": "102", "title": "Second video",
                     "Guid": [{"id": "youtube://yt2"}]}
                ]}}"#,
            )
            .create();
        let repository = repository(&server);

        let items = repository.list_collection_items("c1").unwrap();

        assert_eq!(
            items,
            vec![
                PlexItem {
                    rating_key: "101".to_string(),
                    youtube_video_id: Some("yt1".to_string()),
                },
                PlexItem {
                    rating_key: "102".to_string(),
                    youtube_video_id: Some("yt2".to_string()),
                },
            ]
        );
    }

    #[test]
    fn it_should_create_a_collection_with_alphabetical_sort() {
        let mut server = mockito::Server::new();
        let identity_mock = server
            .mock("GET", "/identity")
            .match_header("accept", "application/json")
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .with_body(r#"{"MediaContainer": {"machineIdentifier": "machine-1"}}"#)
            .expect(1)
            .create();
        let create_mock = server
            .mock("POST", "/library/collections")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("type".into(), "1".into()),
                mockito::Matcher::UrlEncoded("smart".into(), "0".into()),
                mockito::Matcher::UrlEncoded("sectionId".into(), "1".into()),
                mockito::Matcher::UrlEncoded("title".into(), "Lofi beats".into()),
                mockito::Matcher::UrlEncoded(
                    "uri".into(),
                    "server://machine-1/com.plexapp.plugins.library/library/metadata/101,102"
                        .into(),
                ),
            ]))
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .with_body(
                r#"{"MediaContainer": {"Metadata": [{"ratingKey": "c9", "title": "Lofi beats"}]}}"#,
            )
            .expect(2)
            .create();
        let sort_mock = server
            .mock("PUT", "/library/metadata/c9/prefs")
            .match_query(mockito::Matcher::UrlEncoded(
                "collectionSort".into(),
                "1".into(),
            ))
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .expect(2)
            .create();
        let repository = repository(&server);

        let first = repository.create_collection(
            "1",
            "Lofi beats",
            &["101".to_string(), "102".to_string()],
        );
        let second = repository.create_collection(
            "1",
            "Lofi beats",
            &["101".to_string(), "102".to_string()],
        );

        assert_eq!(first.map_err(|e| e.to_string()), Ok(()));
        assert_eq!(second.map_err(|e| e.to_string()), Ok(()));
        identity_mock.assert();
        create_mock.assert();
        sort_mock.assert();
    }

    #[test]
    fn it_should_add_items_to_a_collection() {
        let mut server = mockito::Server::new();
        let _identity_mock = server
            .mock("GET", "/identity")
            .with_status(200)
            .with_body(r#"{"MediaContainer": {"machineIdentifier": "machine-1"}}"#)
            .create();
        let add_mock = server
            .mock("PUT", "/library/collections/c1/items")
            .match_query(mockito::Matcher::UrlEncoded(
                "uri".into(),
                "server://machine-1/com.plexapp.plugins.library/library/metadata/103,104".into(),
            ))
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .create();
        let repository = repository(&server);

        let result = repository.add_items("c1", &["103".to_string(), "104".to_string()]);

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        add_mock.assert();
    }

    #[test]
    fn it_should_remove_an_item_from_a_collection() {
        let mut server = mockito::Server::new();
        let remove_mock = server
            .mock("DELETE", "/library/collections/c1/items/101")
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .create();
        let repository = repository(&server);

        let result = repository.remove_item("c1", "101");

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        remove_mock.assert();
    }

    #[test]
    fn it_should_delete_a_collection() {
        let mut server = mockito::Server::new();
        let delete_mock = server
            .mock("DELETE", "/library/collections/c1")
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .create();
        let repository = repository(&server);

        let result = repository.delete_collection("c1");

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        delete_mock.assert();
    }

    #[test]
    fn it_should_fail_if_the_server_replies_with_an_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/library/sections/1/all")
            .match_query(mockito::Matcher::Any)
            .with_status(500)
            .create();
        let repository = repository(&server);

        let result = repository.list_items("1");

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err(
                "Plex request to /library/sections/1/all failed with status 500 Internal Server Error"
                    .to_string()
            )
        );
    }

    #[test]
    fn it_should_fail_with_the_plex_response_body_if_the_server_replies_with_an_error() {
        let mut server = mockito::Server::new();
        let _identity_mock = server
            .mock("GET", "/identity")
            .with_status(200)
            .with_body(r#"{"MediaContainer": {"machineIdentifier": "machine-1"}}"#)
            .create();
        let _add_mock = server
            .mock("PUT", "/library/collections/c1/items")
            .match_query(mockito::Matcher::Any)
            .with_status(400)
            .with_body("bad uri\n")
            .create();
        let repository = repository(&server);

        let result = repository.add_items("c1", &["103".to_string()]);

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err(
                "Plex request to /library/collections/c1/items failed with status 400 Bad Request: bad uri"
                    .to_string()
            )
        );
    }

    #[test]
    fn it_should_list_sections_with_their_locations() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/library/sections")
            .match_header("accept", "application/json")
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .with_body(
                r#"{"MediaContainer": {"Directory": [
                    {"key": "19", "title": "Kids",
                     "Location": [{"id": 1, "path": "/volume1/media/yarrtube/playlists"}]},
                    {"key": "21", "title": "Channels",
                     "Location": [{"id": 2, "path": "/volume1/media/yarrtube/channels"},
                                  {"id": 3, "path": "/volume2/channels"}]},
                    {"key": "30", "title": "Empty"}
                ]}}"#,
            )
            .create();
        let repository = repository(&server);

        let sections = repository.list_sections().unwrap();

        assert_eq!(
            sections,
            vec![
                PlexSection {
                    id: "19".to_string(),
                    locations: vec!["/volume1/media/yarrtube/playlists".to_string()],
                },
                PlexSection {
                    id: "21".to_string(),
                    locations: vec![
                        "/volume1/media/yarrtube/channels".to_string(),
                        "/volume2/channels".to_string(),
                    ],
                },
                PlexSection {
                    id: "30".to_string(),
                    locations: vec![],
                },
            ]
        );
    }

    #[test]
    fn it_should_scan_a_path_in_a_section() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/library/sections/19/refresh")
            .match_query(mockito::Matcher::UrlEncoded(
                "path".into(),
                "/volume1/media/yarrtube/playlists/kids/Excursió al cinema - Titó".into(),
            ))
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .create();
        let repository = repository(&server);

        let result = repository.scan_path(
            "19",
            "/volume1/media/yarrtube/playlists/kids/Excursió al cinema - Titó",
        );

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        mock.assert();
    }

    #[test]
    fn it_should_fail_to_scan_a_path_if_the_server_replies_with_an_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/library/sections/19/refresh")
            .match_query(mockito::Matcher::Any)
            .with_status(404)
            .create();
        let repository = repository(&server);

        let result = repository.scan_path("19", "/volume1/media/yarrtube/playlists/x");

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err(
                "Plex request to /library/sections/19/refresh failed with status 404 Not Found"
                    .to_string()
            )
        );
    }

    #[test]
    fn it_should_list_match_candidates() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/library/metadata/9804/matches")
            .match_header("accept", "application/json")
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .with_body(
                r#"{"MediaContainer": {"size": 1, "identifier": "com.plexapp.plugins.library",
                    "SearchResult": [
                        {"type": "movie",
                         "guid": "tv.plex.agents.nfo.movie://movie/youtube_h_BOrYxxenc",
                         "name": "Les set cabretes i el llop", "year": 2023}
                    ]}}"#,
            )
            .create();
        let repository = repository(&server);

        let candidates = repository.list_match_candidates("9804").unwrap();

        assert_eq!(
            candidates,
            vec![PlexMatchCandidate {
                guid: "tv.plex.agents.nfo.movie://movie/youtube_h_BOrYxxenc".to_string(),
                name: "Les set cabretes i el llop".to_string(),
            }]
        );
    }

    #[test]
    fn it_should_list_no_match_candidates_if_plex_offers_none() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/library/metadata/9804/matches")
            .with_status(200)
            .with_body(r#"{"MediaContainer": {"size": 0}}"#)
            .create();
        let repository = repository(&server);

        let candidates = repository.list_match_candidates("9804").unwrap();

        assert_eq!(candidates, vec![]);
    }

    #[test]
    fn it_should_match_an_item_to_a_candidate() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("PUT", "/library/metadata/9804/match")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded(
                    "guid".into(),
                    "tv.plex.agents.nfo.movie://movie/youtube_h_BOrYxxenc".into(),
                ),
                mockito::Matcher::UrlEncoded("name".into(), "Les set cabretes i el llop".into()),
            ]))
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .create();
        let repository = repository(&server);

        let result = repository.match_item(
            "9804",
            &PlexMatchCandidate {
                guid: "tv.plex.agents.nfo.movie://movie/youtube_h_BOrYxxenc".to_string(),
                name: "Les set cabretes i el llop".to_string(),
            },
        );

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        mock.assert();
    }

    fn repository(server: &mockito::Server) -> HttpPlexCollectionRepository {
        HttpPlexCollectionRepository::new(PlexConfig {
            base_url: server.url(),
            token: "secret-token".to_string(),
        })
    }
}
