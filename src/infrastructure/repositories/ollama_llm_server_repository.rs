use crate::domain::llm_server::{LlmModel, LlmServer, OllamaUrl};
use anyhow::Context;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::time::Duration;

/// A check is interactive (the "Test connection" button), so a server that
/// doesn't answer fails it quickly.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Port to an LLM server's HTTP API.
pub trait LlmServerRepository: Send + Sync {
    /// The server at `url`: its version and installed models, each with
    /// whether it can think.
    fn find(&self, url: &OllamaUrl) -> anyhow::Result<LlmServer>;
}

/// The Ollama API: `GET /api/version`, `GET /api/tags`, `POST /api/show`.
#[derive(Default)]
pub struct OllamaLlmServerRepository;

impl OllamaLlmServerRepository {
    pub fn new() -> Self {
        Self
    }
}

impl LlmServerRepository for OllamaLlmServerRepository {
    fn find(&self, url: &OllamaUrl) -> anyhow::Result<LlmServer> {
        let client = Self::client()?;
        let version: VersionResponse = Self::get_json(&client, url, "/api/version")?;
        let tags: TagsResponse = Self::get_json(&client, url, "/api/tags")?;
        let models = tags
            .models
            .into_iter()
            .map(|model| Self::model(&client, url, model))
            .collect::<anyhow::Result<_>>()?;
        Ok(LlmServer {
            version: version.version,
            models,
        })
    }
}

impl OllamaLlmServerRepository {
    /// A fresh client per check, as for Plex: a blocking client whose
    /// private thread died would fail every later request.
    fn client() -> anyhow::Result<reqwest::blocking::Client> {
        Ok(reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()?)
    }

    /// `model` with its capabilities (`POST /api/show`).
    fn model(
        client: &reqwest::blocking::Client,
        url: &OllamaUrl,
        model: TagsModel,
    ) -> anyhow::Result<LlmModel> {
        let show: ShowResponse = Self::read_json(
            "/api/show",
            client
                .post(format!("{url}/api/show"))
                .json(&serde_json::json!({ "model": model.name })),
        )?;
        Ok(LlmModel {
            thinking: show.capabilities.iter().any(|c| c == "thinking"),
            name: model.name,
            size_bytes: model.size,
        })
    }

    fn get_json<T: DeserializeOwned>(
        client: &reqwest::blocking::Client,
        url: &OllamaUrl,
        path: &str,
    ) -> anyhow::Result<T> {
        Self::read_json(path, client.get(format!("{url}{path}")))
    }

    fn read_json<T: DeserializeOwned>(
        path: &str,
        request: reqwest::blocking::RequestBuilder,
    ) -> anyhow::Result<T> {
        request
            .send()?
            .error_for_status()?
            .json()
            .with_context(|| format!("{path} didn't answer as Ollama does"))
    }
}

#[derive(Deserialize)]
struct VersionResponse {
    version: String,
}

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<TagsModel>,
}

/// Older Ollama versions don't list capabilities: no thinking then.
#[derive(Deserialize)]
struct ShowResponse {
    #[serde(default)]
    capabilities: Vec<String>,
}

#[derive(Deserialize)]
struct TagsModel {
    name: String,
    size: u64,
}

/// An LLM server answering with a fixed state, or unreachable.
#[cfg(test)]
pub struct FakeLlmServerRepository {
    server: Option<LlmServer>,
}

#[cfg(test)]
impl FakeLlmServerRepository {
    pub fn new(server: LlmServer) -> Self {
        Self {
            server: Some(server),
        }
    }

    /// A server that is down: every call fails.
    pub fn failing() -> Self {
        Self { server: None }
    }
}

#[cfg(test)]
impl LlmServerRepository for FakeLlmServerRepository {
    fn find(&self, _url: &OllamaUrl) -> anyhow::Result<LlmServer> {
        self.server
            .clone()
            .ok_or_else(|| anyhow::anyhow!("connection refused"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm_server::LlmModel;

    #[test]
    fn it_should_read_the_version_and_models() {
        let mut server = mockito::Server::new();
        let _version = server
            .mock("GET", "/api/version")
            .with_body(r#"{"version": "0.40.13"}"#)
            .create();
        let _tags = server
            .mock("GET", "/api/tags")
            .with_body(
                r#"{"models": [{"name": "gemma3:4b", "model": "gemma3:4b", "size": 3338801804}]}"#,
            )
            .create();
        let _show = server
            .mock("POST", "/api/show")
            .match_body(mockito::Matcher::Json(
                serde_json::json!({"model": "gemma3:4b"}),
            ))
            .with_body(r#"{"capabilities": ["completion", "vision"]}"#)
            .create();

        let result = OllamaLlmServerRepository::new().find(&url(&server));

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Ok(LlmServer {
                version: "0.40.13".to_string(),
                models: vec![LlmModel {
                    name: "gemma3:4b".to_string(),
                    size_bytes: 3_338_801_804,
                    thinking: false,
                }],
            })
        );
    }

    #[test]
    fn it_should_read_whether_each_model_can_think() {
        let mut server = mockito::Server::new();
        let _version = server
            .mock("GET", "/api/version")
            .with_body(r#"{"version": "0.40.13"}"#)
            .create();
        let _tags = server
            .mock("GET", "/api/tags")
            .with_body(r#"{"models": [{"name": "qwen3:4b", "size": 2497293918}]}"#)
            .create();
        let _show = server
            .mock("POST", "/api/show")
            .match_body(mockito::Matcher::Json(
                serde_json::json!({"model": "qwen3:4b"}),
            ))
            .with_body(r#"{"capabilities": ["completion", "tools", "thinking"]}"#)
            .create();

        let result = OllamaLlmServerRepository::new().find(&url(&server));

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Ok(LlmServer {
                version: "0.40.13".to_string(),
                models: vec![LlmModel {
                    name: "qwen3:4b".to_string(),
                    size_bytes: 2_497_293_918,
                    thinking: true,
                }],
            })
        );
    }

    #[test]
    fn it_should_fail_if_the_version_endpoint_is_not_json() {
        let mut server = mockito::Server::new();
        let _version = server
            .mock("GET", "/api/version")
            .with_body("<html>Router login</html>")
            .create();

        let result = OllamaLlmServerRepository::new().find(&url(&server));

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err("/api/version didn't answer as Ollama does".to_string())
        );
    }

    #[test]
    fn it_should_fail_if_the_server_replies_with_an_error() {
        let mut server = mockito::Server::new();
        let _version = server
            .mock("GET", "/api/version")
            .with_body(r#"{"version": "0.40.13"}"#)
            .create();
        let _tags = server.mock("GET", "/api/tags").with_status(500).create();

        let result = OllamaLlmServerRepository::new().find(&url(&server));

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err(format!(
                "HTTP status server error (500 Internal Server Error) for url ({}/api/tags)",
                server.url()
            ))
        );
    }

    #[test]
    fn it_should_fail_if_the_server_doesnt_answer_in_time() {
        // Connections queue in the backlog but nobody ever answers.
        let silent = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = OllamaUrl::new(format!("http://{}", silent.local_addr().unwrap())).unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let started = std::time::Instant::now();
        std::thread::spawn(move || {
            let _ = sender.send(OllamaLlmServerRepository::new().find(&url).is_err());
        });

        let failed = receiver.recv_timeout(Duration::from_secs(15));

        assert_eq!(failed, Ok(true));
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    fn url(server: &mockito::Server) -> OllamaUrl {
        OllamaUrl::new(server.url()).unwrap()
    }
}
