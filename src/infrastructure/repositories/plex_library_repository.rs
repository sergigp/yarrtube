use crate::domain::plex::{PlexEpisode, PlexLabels, PlexPlaylist, PlexSection};
use serde::Deserialize;
#[cfg(test)]
use std::sync::Mutex;
use std::time::Duration;

/// The task sharing the serial Light lane must not hold the slot on a hung
/// Plex server, so every request gets an explicit timeout. It leaves room
/// for a large section listing (normally ~1 s) while Plex is busy with its
/// nightly maintenance, which has taken it to 29 s.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// Episodes requested per page of a section listing.
const EPISODES_PAGE_SIZE: usize = 500;

/// Port to the Plex server's library HTTP API, authenticated via
/// `X-Plex-Token`: discovers the library sections, asks one to scan a
/// folder, reads and writes episode labels and manages playlists.
pub trait PlexLibraryRepository: Send + Sync {
    /// Every library section on the server, with the folders it scans.
    fn list_sections(&self) -> anyhow::Result<Vec<PlexSection>>;
    /// Asks Plex to scan only `path` (a server-side folder) in the section,
    /// so a new video is imported without waiting for a library scan.
    fn scan_path(&self, section_id: &str, path: &str) -> anyhow::Result<()>;
    /// The server's machine identifier (`GET /identity`).
    fn server_id(&self) -> anyhow::Result<String>;
    /// Every episode in the section, one per media file
    /// (`GET /library/sections/{id}/all?type=4`, paged).
    fn list_episodes(&self, section_id: &str) -> anyhow::Result<Vec<PlexEpisode>>;
    /// The episode's labels (`GET /library/metadata/{rating_key}`).
    fn find_labels(&self, rating_key: &str) -> anyhow::Result<PlexLabels>;
    /// Writes `labels` to the episode and removes `removed`, locking the
    /// field, or unlocking it when `labels` is empty.
    fn save_labels(
        &self,
        episode: &PlexEpisode,
        labels: &PlexLabels,
        removed: &[String],
    ) -> anyhow::Result<()>;
    /// The token user's video playlists (`GET /playlists?playlistType=video`).
    fn list_playlists(&self) -> anyhow::Result<Vec<PlexPlaylist>>;
    /// Creates a smart playlist of the section's unwatched episodes labelled
    /// `label`, newest air date first.
    fn create_smart_playlist(
        &self,
        server_id: &str,
        section_id: &str,
        label: &str,
        title: &str,
    ) -> anyhow::Result<PlexPlaylist>;
    /// `DELETE /playlists/{rating_key}`.
    fn delete_playlist(&self, rating_key: &str) -> anyhow::Result<()>;
}

pub struct PlexConfig {
    pub base_url: String,
    pub token: String,
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
    #[serde(default)]
    title: String,
    #[serde(rename = "Location", default)]
    locations: Vec<Location>,
}

#[derive(Debug, Deserialize)]
struct Location {
    path: String,
}

#[derive(Debug, Deserialize)]
struct IdentityResponse {
    #[serde(rename = "MediaContainer")]
    media_container: IdentityContainer,
}

#[derive(Debug, Deserialize)]
struct IdentityContainer {
    #[serde(rename = "machineIdentifier")]
    machine_identifier: String,
}

#[derive(Debug, Deserialize)]
struct MetadataResponse {
    #[serde(rename = "MediaContainer")]
    media_container: MetadataContainer,
}

#[derive(Debug, Deserialize)]
struct MetadataContainer {
    #[serde(rename = "totalSize", default)]
    total_size: usize,
    #[serde(rename = "Metadata", default)]
    metadata: Vec<Metadata>,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    #[serde(rename = "ratingKey")]
    rating_key: String,
    #[serde(rename = "Media", default)]
    media: Vec<Media>,
    #[serde(rename = "Label", default)]
    labels: Vec<Tag>,
}

#[derive(Debug, Deserialize)]
struct Tag {
    tag: String,
}

#[derive(Debug, Deserialize)]
struct Media {
    #[serde(rename = "Part", default)]
    parts: Vec<Part>,
}

#[derive(Debug, Deserialize)]
struct Part {
    file: String,
}

pub struct HttpPlexLibraryRepository {
    config: PlexConfig,
    client: reqwest::blocking::Client,
}

impl HttpPlexLibraryRepository {
    pub fn new(config: PlexConfig) -> Self {
        Self {
            config,
            client: reqwest::blocking::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()
                .expect("failed to build the Plex HTTP client"),
        }
    }
}

impl PlexLibraryRepository for HttpPlexLibraryRepository {
    fn list_sections(&self) -> anyhow::Result<Vec<PlexSection>> {
        let response: SectionsResponse = self.get_json("/library/sections", &[])?;
        Ok(response
            .media_container
            .directories
            .into_iter()
            .map(|directory| PlexSection {
                id: directory.key,
                title: directory.title,
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

    fn server_id(&self) -> anyhow::Result<String> {
        let response: IdentityResponse = self.get_json("/identity", &[])?;
        Ok(response.media_container.machine_identifier)
    }

    fn list_episodes(&self, section_id: &str) -> anyhow::Result<Vec<PlexEpisode>> {
        self.list_episodes_from(section_id, 0)
    }

    fn find_labels(&self, rating_key: &str) -> anyhow::Result<PlexLabels> {
        let response: MetadataResponse =
            self.get_json(&format!("/library/metadata/{rating_key}"), &[])?;
        Ok(PlexLabels::new(
            response
                .media_container
                .metadata
                .into_iter()
                .flat_map(|metadata| metadata.labels)
                .map(|label| label.tag)
                .collect(),
        ))
    }

    fn save_labels(
        &self,
        episode: &PlexEpisode,
        labels: &PlexLabels,
        removed: &[String],
    ) -> anyhow::Result<()> {
        let endpoint = format!("/library/sections/{}/all", episode.section_id);
        let response = self
            .client
            .put(format!("{}{endpoint}", self.config.base_url))
            .query(&label_edit_query(episode, labels, removed))
            .header("Accept", "application/json")
            .header("X-Plex-Token", &self.config.token)
            .send()?;
        Self::ensure_success(&endpoint, response).map(drop)
    }

    fn list_playlists(&self) -> anyhow::Result<Vec<PlexPlaylist>> {
        Ok(Vec::new())
    }

    fn create_smart_playlist(
        &self,
        _server_id: &str,
        _section_id: &str,
        _label: &str,
        _title: &str,
    ) -> anyhow::Result<PlexPlaylist> {
        Ok(PlexPlaylist {
            rating_key: String::new(),
            title: String::new(),
        })
    }

    fn delete_playlist(&self, _rating_key: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

impl HttpPlexLibraryRepository {
    /// The section's episodes from `start` on, one page per request.
    fn list_episodes_from(
        &self,
        section_id: &str,
        start: usize,
    ) -> anyhow::Result<Vec<PlexEpisode>> {
        let start_param = start.to_string();
        let page_size = EPISODES_PAGE_SIZE.to_string();
        let page: MetadataResponse = self.get_json(
            &format!("/library/sections/{section_id}/all"),
            &[
                ("type", "4"),
                ("X-Plex-Container-Start", &start_param),
                ("X-Plex-Container-Size", &page_size),
            ],
        )?;
        let container = page.media_container;
        let next = start + container.metadata.len();
        let episodes: Vec<PlexEpisode> = container
            .metadata
            .into_iter()
            .flat_map(|metadata| episodes_of(section_id, metadata))
            .collect();
        if next == start || next >= container.total_size {
            return Ok(episodes);
        }
        Ok([episodes, self.list_episodes_from(section_id, next)?].concat())
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

/// The query of a label edit: each label added by index, the removed ones
/// in one comma-joined `-=` param, and the field locked so Plex's agents
/// never overwrite it, or unlocked once it holds no label.
fn label_edit_query(
    episode: &PlexEpisode,
    labels: &PlexLabels,
    removed: &[String],
) -> Vec<(String, String)> {
    let added = labels
        .names()
        .iter()
        .enumerate()
        .map(|(i, name)| (format!("label[{i}].tag.tag"), name.clone()));
    let locked = if labels.names().is_empty() { "0" } else { "1" };
    let removed =
        (!removed.is_empty()).then(|| ("label[].tag.tag-".to_string(), removed.join(",")));
    [
        ("type".to_string(), "4".to_string()),
        ("id".to_string(), episode.rating_key.clone()),
    ]
    .into_iter()
    .chain(added)
    .chain(removed)
    .chain([("label.locked".to_string(), locked.to_string())])
    .collect()
}

/// One episode per file of `metadata`'s media parts.
fn episodes_of(section_id: &str, metadata: Metadata) -> Vec<PlexEpisode> {
    metadata
        .media
        .into_iter()
        .flat_map(|media| media.parts)
        .map(|part| PlexEpisode {
            rating_key: metadata.rating_key.clone(),
            section_id: section_id.to_string(),
            file: part.file,
        })
        .collect()
}

/// Plex server fake: sections with their titles and folders, the episodes
/// in them with their labels, and the token user's playlists.
#[cfg(test)]
#[derive(Default)]
pub struct FakePlexLibraryRepository {
    sections: Mutex<std::collections::BTreeMap<String, FakeSection>>,
    episodes: Mutex<Vec<FakeEpisode>>,
    playlists: Mutex<Vec<FakePlaylist>>,
    /// Every write requested, in order, e.g. `scan:19:/volume1/x`,
    /// `labels:101:Chess,Funny:-Old`, `create_playlist:19:Chess:Chess`,
    /// `delete_playlist:pl1`.
    mutations: Mutex<Vec<String>>,
    unreachable: bool,
    /// Rating keys whose label writes Plex rejects.
    rejected: Vec<String>,
}

#[cfg(test)]
#[derive(Default)]
struct FakeSection {
    title: String,
    locations: Vec<String>,
}

#[cfg(test)]
struct FakeEpisode {
    episode: PlexEpisode,
    labels: Vec<String>,
}

/// A playlist on the fake server; `filter` is the section and label of a
/// smart playlist the fake created, `None` for one it was seeded with.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakePlaylist {
    pub rating_key: String,
    pub title: String,
    pub filter: Option<(String, String)>,
}

#[cfg(test)]
impl FakePlexLibraryRepository {
    pub const SERVER_ID: &str = "fake-server";

    /// The same fake with `location` added to `section_id`'s folders.
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

    /// The same fake with `section_id` titled `title`.
    pub fn with_section_title(self, section_id: &str, title: &str) -> Self {
        self.sections
            .lock()
            .unwrap()
            .entry(section_id.to_string())
            .or_default()
            .title = title.to_string();
        self
    }

    /// The same fake with an episode for `file` in `section_id`, labelled
    /// `labels`.
    pub fn with_episode(
        self,
        section_id: &str,
        rating_key: &str,
        file: &str,
        labels: &[&str],
    ) -> Self {
        self.episodes.lock().unwrap().push(FakeEpisode {
            episode: PlexEpisode {
                rating_key: rating_key.to_string(),
                section_id: section_id.to_string(),
                file: file.to_string(),
            },
            labels: labels.iter().map(|label| label.to_string()).collect(),
        });
        self
    }

    /// The same fake with a playlist yarrtube did not create.
    pub fn with_playlist(self, rating_key: &str, title: &str) -> Self {
        self.playlists.lock().unwrap().push(FakePlaylist {
            rating_key: rating_key.to_string(),
            title: title.to_string(),
            filter: None,
        });
        self
    }

    /// The same fake rejecting every label write to `rating_key`.
    pub fn rejecting(self, rating_key: &str) -> Self {
        Self {
            rejected: [self.rejected.clone(), vec![rating_key.to_string()]].concat(),
            ..self
        }
    }

    /// A Plex server that is down: every call fails.
    pub fn failing() -> Self {
        Self {
            unreachable: true,
            ..Self::default()
        }
    }

    pub fn mutations(&self) -> Vec<String> {
        self.mutations.lock().unwrap().clone()
    }

    /// The labels of the episode `rating_key`, in the order written.
    pub fn labels(&self, rating_key: &str) -> Vec<String> {
        self.episodes
            .lock()
            .unwrap()
            .iter()
            .find(|fake| fake.episode.rating_key == rating_key)
            .map(|fake| fake.labels.clone())
            .unwrap_or_default()
    }

    pub fn playlists(&self) -> Vec<FakePlaylist> {
        self.playlists.lock().unwrap().clone()
    }

    fn ensure_reachable(&self) -> anyhow::Result<()> {
        if self.unreachable {
            anyhow::bail!("Plex is unreachable");
        }
        Ok(())
    }

    fn record(&self, mutation: String) {
        self.mutations.lock().unwrap().push(mutation);
    }
}

#[cfg(test)]
impl PlexLibraryRepository for FakePlexLibraryRepository {
    fn list_sections(&self) -> anyhow::Result<Vec<PlexSection>> {
        self.ensure_reachable()?;
        Ok(self
            .sections
            .lock()
            .unwrap()
            .iter()
            .map(|(id, section)| PlexSection {
                id: id.clone(),
                title: section.title.clone(),
                locations: section.locations.clone(),
            })
            .collect())
    }

    fn scan_path(&self, section_id: &str, path: &str) -> anyhow::Result<()> {
        self.ensure_reachable()?;
        self.record(format!("scan:{section_id}:{path}"));
        Ok(())
    }

    fn server_id(&self) -> anyhow::Result<String> {
        self.ensure_reachable()?;
        Ok(Self::SERVER_ID.to_string())
    }

    fn list_episodes(&self, section_id: &str) -> anyhow::Result<Vec<PlexEpisode>> {
        self.ensure_reachable()?;
        Ok(self
            .episodes
            .lock()
            .unwrap()
            .iter()
            .filter(|fake| fake.episode.section_id == section_id)
            .map(|fake| fake.episode.clone())
            .collect())
    }

    fn find_labels(&self, rating_key: &str) -> anyhow::Result<PlexLabels> {
        self.ensure_reachable()?;
        Ok(PlexLabels::new(self.labels(rating_key)))
    }

    fn save_labels(
        &self,
        episode: &PlexEpisode,
        labels: &PlexLabels,
        removed: &[String],
    ) -> anyhow::Result<()> {
        self.ensure_reachable()?;
        if self.rejected.contains(&episode.rating_key) {
            anyhow::bail!("Plex rejected the labels of {}", episode.rating_key);
        }
        self.record(format!(
            "labels:{}:{}:-{}",
            episode.rating_key,
            labels.names().join(","),
            removed.join(",")
        ));
        self.episodes
            .lock()
            .unwrap()
            .iter_mut()
            .filter(|fake| fake.episode.rating_key == episode.rating_key)
            .for_each(|fake| fake.labels = labels.names().to_vec());
        Ok(())
    }

    fn list_playlists(&self) -> anyhow::Result<Vec<PlexPlaylist>> {
        self.ensure_reachable()?;
        Ok(self
            .playlists()
            .into_iter()
            .map(|playlist| PlexPlaylist {
                rating_key: playlist.rating_key,
                title: playlist.title,
            })
            .collect())
    }

    fn create_smart_playlist(
        &self,
        server_id: &str,
        section_id: &str,
        label: &str,
        title: &str,
    ) -> anyhow::Result<PlexPlaylist> {
        self.ensure_reachable()?;
        anyhow::ensure!(server_id == Self::SERVER_ID, "unknown server {server_id}");
        self.record(format!("create_playlist:{section_id}:{label}:{title}"));
        let mut playlists = self.playlists.lock().unwrap();
        let playlist = FakePlaylist {
            rating_key: format!("pl{}", playlists.len() + 1),
            title: title.to_string(),
            filter: Some((section_id.to_string(), label.to_string())),
        };
        playlists.push(playlist.clone());
        Ok(PlexPlaylist {
            rating_key: playlist.rating_key,
            title: playlist.title,
        })
    }

    fn delete_playlist(&self, rating_key: &str) -> anyhow::Result<()> {
        self.ensure_reachable()?;
        self.record(format!("delete_playlist:{rating_key}"));
        self.playlists
            .lock()
            .unwrap()
            .retain(|playlist| playlist.rating_key != rating_key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_fail_if_the_server_replies_with_an_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/library/sections")
            .with_status(500)
            .create();
        let repository = repository(&server);

        let result = repository.list_sections();

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err(
                "Plex request to /library/sections failed with status 500 Internal Server Error"
                    .to_string()
            )
        );
    }

    #[test]
    fn it_should_fail_with_the_plex_response_body_if_the_server_replies_with_an_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/library/sections/19/refresh")
            .match_query(mockito::Matcher::Any)
            .with_status(400)
            .with_body("bad path\n")
            .create();
        let repository = repository(&server);

        let result = repository.scan_path("19", "/volume1/media/yarrtube/playlists/x");

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err(
                "Plex request to /library/sections/19/refresh failed with status 400 Bad Request: bad path"
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
                    title: "Kids".to_string(),
                    locations: vec!["/volume1/media/yarrtube/playlists".to_string()],
                },
                PlexSection {
                    id: "21".to_string(),
                    title: "Channels".to_string(),
                    locations: vec![
                        "/volume1/media/yarrtube/channels".to_string(),
                        "/volume2/channels".to_string(),
                    ],
                },
                PlexSection {
                    id: "30".to_string(),
                    title: "Empty".to_string(),
                    locations: vec![],
                },
            ]
        );
    }

    #[test]
    fn it_should_list_sections_with_their_titles() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/library/sections")
            .with_status(200)
            .with_body(
                r#"{"MediaContainer": {"Directory": [
                    {"key": "19", "title": "Sergi's Playlists", "Location": []}
                ]}}"#,
            )
            .create();
        let repository = repository(&server);

        let sections = repository.list_sections().unwrap();

        assert_eq!(
            sections,
            vec![PlexSection {
                id: "19".to_string(),
                title: "Sergi's Playlists".to_string(),
                locations: vec![],
            }]
        );
    }

    #[test]
    fn it_should_read_the_server_id() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/identity")
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .with_body(
                r#"{"MediaContainer": {"size": 0, "claimed": true,
                    "machineIdentifier": "abc123", "version": "1.43.4"}}"#,
            )
            .create();
        let repository = repository(&server);

        let server_id = repository.server_id();

        assert_eq!(
            server_id.map_err(|e| e.to_string()),
            Ok("abc123".to_string())
        );
    }

    #[test]
    fn it_should_list_a_section_episodes_with_their_files() {
        let mut server = mockito::Server::new();
        let _first_page = server
            .mock("GET", "/library/sections/19/all")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("type".into(), "4".into()),
                mockito::Matcher::UrlEncoded("X-Plex-Container-Start".into(), "0".into()),
            ]))
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .with_body(
                r#"{"MediaContainer": {"size": 2, "totalSize": 3, "offset": 0, "Metadata": [
                    {"ratingKey": "101", "Media": [{"Part": [{"file": "/volume1/a.mp4"}]}]},
                    {"ratingKey": "102", "Media": [
                        {"Part": [{"file": "/volume1/b1.mp4"}, {"file": "/volume1/b2.mp4"}]},
                        {"Part": [{"file": "/volume1/b.mkv"}]}
                    ]}
                ]}}"#,
            )
            .create();
        let _second_page = server
            .mock("GET", "/library/sections/19/all")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("type".into(), "4".into()),
                mockito::Matcher::UrlEncoded("X-Plex-Container-Start".into(), "2".into()),
            ]))
            .with_status(200)
            .with_body(
                r#"{"MediaContainer": {"size": 1, "totalSize": 3, "offset": 2, "Metadata": [
                    {"ratingKey": "103", "Media": [{"Part": [{"file": "/volume1/c.mp4"}]}]}
                ]}}"#,
            )
            .create();
        let repository = repository(&server);

        let episodes = repository.list_episodes("19");

        assert_eq!(
            episodes.map_err(|e| e.to_string()),
            Ok(vec![
                episode("101", "/volume1/a.mp4"),
                episode("102", "/volume1/b1.mp4"),
                episode("102", "/volume1/b2.mp4"),
                episode("102", "/volume1/b.mkv"),
                episode("103", "/volume1/c.mp4"),
            ])
        );
    }

    #[test]
    fn it_should_read_an_episode_labels() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/library/metadata/101")
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .with_body(
                r#"{"MediaContainer": {"size": 1, "Metadata": [
                    {"ratingKey": "101", "Label": [{"id": 7, "tag": "Funny"}, {"id": 8, "tag": "Chess"}]}
                ]}}"#,
            )
            .create();
        let repository = repository(&server);

        let labels = repository.find_labels("101");

        assert_eq!(
            labels.map_err(|e| e.to_string()),
            Ok(PlexLabels::new(vec![
                "Chess".to_string(),
                "Funny".to_string()
            ]))
        );
    }

    #[test]
    fn it_should_save_labels_removing_the_others() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("PUT", "/library/sections/19/all")
            .match_query(mockito::Matcher::Exact(
                "type=4&id=101&label%5B0%5D.tag.tag=Chess&label%5B1%5D.tag.tag=M%C3%BAsica\
                 &label%5B%5D.tag.tag-=Old%2COther+one&label.locked=1"
                    .to_string(),
            ))
            .match_header("x-plex-token", "secret-token")
            .with_status(200)
            .create();
        let repository = repository(&server);

        let result = repository.save_labels(
            &episode("101", "/volume1/a.mp4"),
            &PlexLabels::new(vec!["Chess".to_string(), "Música".to_string()]),
            &["Old".to_string(), "Other one".to_string()],
        );

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        mock.assert();
    }

    #[test]
    fn it_should_clear_labels_and_unlock_them() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("PUT", "/library/sections/19/all")
            .match_query(mockito::Matcher::Exact(
                "type=4&id=101&label%5B%5D.tag.tag-=Chess&label.locked=0".to_string(),
            ))
            .with_status(200)
            .create();
        let repository = repository(&server);

        let result = repository.save_labels(
            &episode("101", "/volume1/a.mp4"),
            &PlexLabels::none(),
            &["Chess".to_string()],
        );

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        mock.assert();
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

    fn episode(rating_key: &str, file: &str) -> PlexEpisode {
        PlexEpisode {
            rating_key: rating_key.to_string(),
            section_id: "19".to_string(),
            file: file.to_string(),
        }
    }

    fn repository(server: &mockito::Server) -> HttpPlexLibraryRepository {
        HttpPlexLibraryRepository::new(PlexConfig {
            base_url: server.url(),
            token: "secret-token".to_string(),
        })
    }
}
