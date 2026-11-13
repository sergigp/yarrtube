use crate::domain::llm_server::{
    LlmChat, LlmModel, LlmReply, LlmServer, LlmServerUnreachable, OllamaUrl,
};
use anyhow::Context;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::time::Duration;

/// A check is interactive (the "Test connection" button), so a server that
/// doesn't answer fails it quickly.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// The Ollama API: `GET /api/version`, `GET /api/tags`, `POST /api/show`.
pub struct OllamaLlmServerRepository {
    check_timeout: Duration,
}

impl OllamaLlmServerRepository {
    pub fn new() -> Self {
        Self {
            check_timeout: REQUEST_TIMEOUT,
        }
    }

    /// The server at `url`: its version and installed models, each with
    /// whether it can think.
    pub fn find(&self, url: &OllamaUrl) -> anyhow::Result<LlmServer> {
        let client = Self::client(self.check_timeout)?;
        let version: VersionResponse = Self::get_json(&client, url, "/api/version")?;
        let tags: TagsResponse = Self::get_json(&client, url, "/api/tags")?;
        let models = tags
            .models
            .into_iter()
            .map(|model| Self::model(&client, url, model))
            .collect::<anyhow::Result<_>>()?;
        Ok(LlmServer {
            version: Some(version.version),
            models,
        })
    }

    /// What the model says to `chat`: `POST /api/chat`, unstreamed.
    pub fn chat(&self, url: &OllamaUrl, chat: &LlmChat) -> anyhow::Result<LlmReply> {
        let client = Self::client(chat.timeout)?;
        let sent = client
            .post(format!("{url}/api/chat"))
            .json(&Self::chat_body(chat))
            .send();
        let response = match sent {
            Err(e) if e.is_timeout() && !e.is_connect() => {
                let minutes = chat.timeout.as_secs() / 60;
                return Ok(LlmReply::refused(format!(
                    "no answer within {minutes} minutes"
                )));
            }
            sent => sent.map_err(Self::send_error)?,
        };
        if response.status() == StatusCode::BAD_REQUEST {
            return Ok(Self::refused(Self::refusal(response)?));
        }
        let response: ChatResponse = response
            .error_for_status()?
            .json()
            .context("/api/chat didn't answer as Ollama does")?;
        Ok(LlmReply {
            content: response.message.content,
            thinking: response.message.thinking,
            input_tokens: response.prompt_eval_count,
            output_tokens: response.eval_count,
            stopped_at_limit: response.done_reason.as_deref() == Some("length"),
            refusal: None,
            exceeded_context: false,
        })
    }
}

impl Default for OllamaLlmServerRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl OllamaLlmServerRepository {
    /// A check that gives up after `timeout` instead of `REQUEST_TIMEOUT`, so
    /// tests of a server that never answers don't wait seconds for it.
    #[cfg(test)]
    fn with_check_timeout(timeout: Duration) -> Self {
        Self {
            check_timeout: timeout,
        }
    }

    /// A fresh client per check, as for Plex: a blocking client whose
    /// private thread died would fail every later request.
    fn client(timeout: Duration) -> anyhow::Result<reqwest::blocking::Client> {
        Ok(reqwest::blocking::Client::builder()
            .connect_timeout(REQUEST_TIMEOUT)
            .timeout(timeout)
            .build()?)
    }

    /// `error`, tagged `LlmServerUnreachable` when it is a connect error.
    fn send_error(error: reqwest::Error) -> anyhow::Error {
        let connect = error.is_connect();
        let error = anyhow::Error::new(error);
        if connect {
            error.context(LlmServerUnreachable)
        } else {
            error
        }
    }

    /// A refusal as `error`, too long when Ollama says the prompt exceeds
    /// the context.
    fn refused(error: String) -> LlmReply {
        let too_long = [
            "exceeds the available context size",
            "exceeds the context length",
        ]
        .iter()
        .any(|wording| error.contains(wording));
        match too_long {
            true => LlmReply::refused_as_too_long(error),
            false => LlmReply::refused(error),
        }
    }

    fn refusal(response: reqwest::blocking::Response) -> anyhow::Result<String> {
        let refusal: ChatRefusal = response
            .json()
            .context("/api/chat didn't answer as Ollama does")?;
        Ok(refusal.error)
    }

    /// The request body; `format` only when the chat has a schema.
    fn chat_body(chat: &LlmChat) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": chat.model,
            "stream": false,
            "truncate": false,
            "shift": false,
            "think": chat.thinking,
            "options": {
                "num_ctx": chat.context_length,
                "temperature": chat.temperature,
                "num_predict": chat.max_output_tokens,
            },
            "messages": [
                {"role": "system", "content": chat.system},
                {"role": "user", "content": chat.user},
            ],
        });
        if let Some(schema) = &chat.format {
            body["format"] = schema.clone();
        }
        body
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
            display_name: None,
            size_bytes: Some(model.size),
            context_window: None,
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
            .send()
            .map_err(Self::send_error)?
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
struct ChatRefusal {
    error: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatMessage,
    #[serde(default)]
    prompt_eval_count: u32,
    #[serde(default)]
    eval_count: u32,
    done_reason: Option<String>,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
    /// Only when the chat asked for thinking.
    #[serde(default)]
    thinking: String,
}

#[derive(Deserialize)]
struct TagsModel {
    name: String,
    size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm_server::{Effort, LlmModel, MIN_CHAT_TIMEOUT};

    #[test]
    fn it_should_post_the_chat_with_its_options() {
        let mut server = mockito::Server::new();
        let chat_mock = server
            .mock("POST", "/api/chat")
            .match_body(mockito::Matcher::Json(serde_json::json!({
                "model": "qwen3:4b",
                "stream": false,
                "truncate": false,
                "shift": false,
                "think": true,
                "options": {"num_ctx": 12288, "temperature": 0.3, "num_predict": 3288},
                "messages": [
                    {"role": "system", "content": "Be brief."},
                    {"role": "user", "content": "Title: Sicilian"},
                ],
            })))
            .with_body(ANSWER)
            .create();

        let result = OllamaLlmServerRepository::new().chat(&url(&server), &chat());

        assert_eq!(result.map(|_| ()).map_err(|e| e.to_string()), Ok(()));
        chat_mock.assert();
    }

    #[test]
    fn it_should_send_the_response_schema_as_format() {
        let mut server = mockito::Server::new();
        let schema = serde_json::json!({
            "type": "object",
            "properties": {"belongs": {"type": "boolean"}},
            "required": ["belongs"],
        });
        let chat_mock = server
            .mock("POST", "/api/chat")
            .match_body(mockito::Matcher::PartialJson(
                serde_json::json!({ "format": schema.clone() }),
            ))
            .with_body(ANSWER)
            .create();
        let with_schema = LlmChat {
            format: Some(schema),
            ..chat()
        };

        let result = OllamaLlmServerRepository::new().chat(&url(&server), &with_schema);

        assert_eq!(result.map(|_| ()).map_err(|e| e.to_string()), Ok(()));
        chat_mock.assert();
    }

    #[test]
    fn it_should_read_the_answer_and_tokens() {
        let mut server = mockito::Server::new();
        let _chat = server.mock("POST", "/api/chat").with_body(ANSWER).create();

        let reply = OllamaLlmServerRepository::new()
            .chat(&url(&server), &chat())
            .unwrap();

        assert_eq!(
            reply,
            LlmReply {
                content: "Three games.".to_string(),
                thinking: String::new(),
                input_tokens: 120,
                output_tokens: 250,
                stopped_at_limit: false,
                refusal: None,
                exceeded_context: false,
            }
        );
    }

    #[test]
    fn it_should_read_the_thinking() {
        let mut server = mockito::Server::new();
        let _chat = server
            .mock("POST", "/api/chat")
            .with_body(
                r#"{"message": {"content": "", "thinking": "Is it Tech? Maybe."}, "done_reason": "length", "prompt_eval_count": 120, "eval_count": 3072}"#,
            )
            .create();

        let reply = OllamaLlmServerRepository::new()
            .chat(&url(&server), &chat())
            .unwrap();

        assert_eq!(
            reply,
            LlmReply {
                content: String::new(),
                thinking: "Is it Tech? Maybe.".to_string(),
                input_tokens: 120,
                output_tokens: 3072,
                stopped_at_limit: true,
                refusal: None,
                exceeded_context: false,
            }
        );
    }

    #[test]
    fn it_should_report_a_reply_stopped_at_the_limit() {
        let mut server = mockito::Server::new();
        let _chat = server
            .mock("POST", "/api/chat")
            .with_body(r#"{"message": {"content": "Three"}, "done_reason": "length"}"#)
            .create();

        let reply = OllamaLlmServerRepository::new()
            .chat(&url(&server), &chat())
            .unwrap();

        assert!(reply.stopped_at_limit);
    }

    #[test]
    fn it_should_report_a_refusal_on_a_bad_request() {
        let mut server = mockito::Server::new();
        let _chat = server
            .mock("POST", "/api/chat")
            .with_status(400)
            .with_body(r#"{"error": "invalid format: expected \"json\" or a JSON schema"}"#)
            .create();

        let reply = OllamaLlmServerRepository::new()
            .chat(&url(&server), &chat())
            .unwrap();

        assert_eq!(
            reply,
            LlmReply::refused("invalid format: expected \"json\" or a JSON schema")
        );
    }

    #[test]
    fn it_should_report_a_refusal_as_too_long_when_the_prompt_exceeds_the_context() {
        let mut server = mockito::Server::new();
        let _chat = server
            .mock("POST", "/api/chat")
            .with_status(400)
            .with_body(r#"{"error": "request (7109 tokens) exceeds the available context size (512 tokens), try increasing it"}"#)
            .create();

        let reply = OllamaLlmServerRepository::new()
            .chat(&url(&server), &chat())
            .unwrap();

        assert_eq!(
            reply,
            LlmReply::refused_as_too_long(
                "request (7109 tokens) exceeds the available context size (512 tokens), try increasing it"
            )
        );
    }

    #[test]
    fn it_should_report_a_refusal_as_too_long_when_the_input_length_exceeds_the_context() {
        let mut server = mockito::Server::new();
        let _chat = server
            .mock("POST", "/api/chat")
            .with_status(400)
            .with_body(r#"{"error": "the input length exceeds the context length"}"#)
            .create();

        let reply = OllamaLlmServerRepository::new()
            .chat(&url(&server), &chat())
            .unwrap();

        assert_eq!(
            reply,
            LlmReply::refused_as_too_long("the input length exceeds the context length")
        );
    }

    #[test]
    fn it_should_report_a_refusal_when_the_answer_takes_too_long() {
        // Connections queue in the backlog but nobody ever answers.
        let silent = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = OllamaUrl::new(format!("http://{}", silent.local_addr().unwrap())).unwrap();
        let chat = LlmChat {
            timeout: Duration::from_millis(100),
            ..chat()
        };

        let reply = OllamaLlmServerRepository::new().chat(&url, &chat).unwrap();

        assert_eq!(reply, LlmReply::refused("no answer within 0 minutes"));
    }

    #[test]
    fn it_should_report_an_unreachable_server_on_chat() {
        let url = closed_url();

        let result = OllamaLlmServerRepository::new().chat(&url, &chat());

        assert_eq!(
            result.map_err(|e| e.is::<LlmServerUnreachable>()),
            Err(true)
        );
    }

    #[test]
    fn it_should_report_an_unreachable_server_on_find() {
        let url = closed_url();

        let result = OllamaLlmServerRepository::new().find(&url);

        assert_eq!(
            result.map_err(|e| e.is::<LlmServerUnreachable>()),
            Err(true)
        );
    }

    #[test]
    fn it_should_not_report_a_dropped_connection_as_unreachable() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = OllamaUrl::new(format!("http://{}", listener.local_addr().unwrap())).unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                drop(stream);
            }
        });

        let result = OllamaLlmServerRepository::new().chat(&url, &chat());

        assert_eq!(
            result.map_err(|e| e.is::<LlmServerUnreachable>()),
            Err(false)
        );
    }

    #[test]
    fn it_should_fail_on_an_error_status() {
        let mut server = mockito::Server::new();
        let _chat = server.mock("POST", "/api/chat").with_status(500).create();

        let result = OllamaLlmServerRepository::new().chat(&url(&server), &chat());

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err(format!(
                "HTTP status server error (500 Internal Server Error) for url ({}/api/chat)",
                server.url()
            ))
        );
    }

    #[test]
    fn it_should_fail_if_the_model_is_not_found() {
        let mut server = mockito::Server::new();
        let _chat = server.mock("POST", "/api/chat").with_status(404).create();

        let result = OllamaLlmServerRepository::new().chat(&url(&server), &chat());

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err(format!(
                "HTTP status client error (404 Not Found) for url ({}/api/chat)",
                server.url()
            ))
        );
    }

    #[test]
    fn it_should_fail_on_an_answer_that_is_not_ollamas() {
        let mut server = mockito::Server::new();
        let _chat = server
            .mock("POST", "/api/chat")
            .with_body("<html>not ollama</html>")
            .create();

        let result = OllamaLlmServerRepository::new().chat(&url(&server), &chat());

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err("/api/chat didn't answer as Ollama does".to_string())
        );
    }

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
                version: Some("0.40.13".to_string()),
                models: vec![LlmModel {
                    name: "gemma3:4b".to_string(),
                    display_name: None,
                    size_bytes: Some(3_338_801_804),
                    thinking: false,
                    context_window: None,
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
                version: Some("0.40.13".to_string()),
                models: vec![LlmModel {
                    name: "qwen3:4b".to_string(),
                    display_name: None,
                    size_bytes: Some(2_497_293_918),
                    thinking: true,
                    context_window: None,
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
    fn it_should_not_report_a_slow_server_check_as_unreachable() {
        // Connections queue in the backlog but nobody ever answers.
        let silent = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = OllamaUrl::new(format!("http://{}", silent.local_addr().unwrap())).unwrap();

        let result =
            OllamaLlmServerRepository::with_check_timeout(Duration::from_millis(100)).find(&url);

        assert_eq!(
            result.map_err(|e| e.is::<LlmServerUnreachable>()),
            Err(false)
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
            let repository =
                OllamaLlmServerRepository::with_check_timeout(Duration::from_millis(100));
            let _ = sender.send(repository.find(&url).is_err());
        });

        let failed = receiver.recv_timeout(Duration::from_secs(5));

        assert_eq!(failed, Ok(true));
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    /// A URL where nothing listens: connecting is refused.
    fn closed_url() -> OllamaUrl {
        let closed = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        OllamaUrl::new(format!("http://{closed}")).unwrap()
    }

    fn url(server: &mockito::Server) -> OllamaUrl {
        OllamaUrl::new(server.url()).unwrap()
    }

    const ANSWER: &str = r#"{"message": {"role": "assistant", "content": "Three games."}, "done_reason": "stop", "prompt_eval_count": 120, "eval_count": 250}"#;

    fn chat() -> LlmChat {
        LlmChat {
            model: "qwen3:4b".to_string(),
            system: "Be brief.".to_string(),
            user: "Title: Sicilian".to_string(),
            context_length: 12288,
            temperature: 0.3,
            thinking: true,
            effort: Effort::default(),
            max_output_tokens: 3288,
            format: None,
            timeout: MIN_CHAT_TIMEOUT,
        }
    }
}
