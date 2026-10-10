use crate::domain::plex::PlexSection;
use serde::Deserialize;
#[cfg(test)]
use std::sync::Mutex;
use std::time::Duration;

/// The task sharing the serial Light lane must not hold the slot on a hung
/// Plex server, so every request gets an explicit timeout. It leaves room
/// for a large section listing (normally ~1 s) while Plex is busy with its
/// nightly maintenance, which has taken it to 29 s.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Port to the Plex server's library HTTP API, authenticated via
/// `X-Plex-Token`: discovers the library sections and asks one to scan a
/// folder.
pub trait PlexLibraryRepository: Send + Sync {
    /// Every library section on the server, with the folders it scans.
    fn list_sections(&self) -> anyhow::Result<Vec<PlexSection>>;
    /// Asks Plex to scan only `path` (a server-side folder) in the section,
    /// so a new video is imported without waiting for a library scan.
    fn scan_path(&self, section_id: &str, path: &str) -> anyhow::Result<()>;
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
    #[serde(rename = "Location", default)]
    locations: Vec<Location>,
}

#[derive(Debug, Deserialize)]
struct Location {
    path: String,
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
}

impl HttpPlexLibraryRepository {
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

/// Plex server fake: each section id maps to the folders it scans.
#[cfg(test)]
#[derive(Default)]
pub struct FakePlexLibraryRepository {
    sections: Mutex<std::collections::BTreeMap<String, Vec<String>>>,
    /// Every scan requested, in order, e.g. `scan:19:/volume1/x`.
    mutations: Mutex<Vec<String>>,
    unreachable: bool,
}

#[cfg(test)]
impl FakePlexLibraryRepository {
    /// The same fake with `location` added to `section_id`'s folders.
    pub fn with_location(self, section_id: &str, location: &str) -> Self {
        self.sections
            .lock()
            .unwrap()
            .entry(section_id.to_string())
            .or_default()
            .push(location.to_string());
        self
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
}

#[cfg(test)]
impl PlexLibraryRepository for FakePlexLibraryRepository {
    fn list_sections(&self) -> anyhow::Result<Vec<PlexSection>> {
        if self.unreachable {
            anyhow::bail!("Plex is unreachable");
        }
        Ok(self
            .sections
            .lock()
            .unwrap()
            .iter()
            .map(|(id, locations)| PlexSection {
                id: id.clone(),
                locations: locations.clone(),
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

    fn repository(server: &mockito::Server) -> HttpPlexLibraryRepository {
        HttpPlexLibraryRepository::new(PlexConfig {
            base_url: server.url(),
            token: "secret-token".to_string(),
        })
    }
}
