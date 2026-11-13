use crate::domain::llm_server::{
    LlmChat, LlmConnection, LlmReply, LlmServer, LlmServerUnreachable,
};
use crate::infrastructure::repositories::anthropic_llm_server_repository::AnthropicLlmServerRepository;
use crate::infrastructure::repositories::ollama_llm_server_repository::OllamaLlmServerRepository;

/// Port to the LLM provider AI features run on.
pub trait LlmServerRepository: Send + Sync {
    /// The provider behind `connection` and its models: Ollama's installed
    /// ones, each with whether it can think, or the ones Anthropic lists.
    fn find(&self, connection: &LlmConnection) -> anyhow::Result<LlmServer>;
    /// What the model says to `chat`.
    fn chat(&self, connection: &LlmConnection, chat: &LlmChat) -> anyhow::Result<LlmReply>;
}

/// Sends each connection to its provider's adapter.
pub struct RoutingLlmServerRepository {
    ollama: OllamaLlmServerRepository,
    /// `None` without `YARRTUBE_ANTHROPIC_API_KEY`: Anthropic is unavailable.
    anthropic: Option<AnthropicLlmServerRepository>,
}

impl RoutingLlmServerRepository {
    pub fn new(
        ollama: OllamaLlmServerRepository,
        anthropic: Option<AnthropicLlmServerRepository>,
    ) -> Self {
        Self { ollama, anthropic }
    }
}

impl LlmServerRepository for RoutingLlmServerRepository {
    fn find(&self, connection: &LlmConnection) -> anyhow::Result<LlmServer> {
        match connection {
            LlmConnection::Ollama(url) => self.ollama.find(url),
            LlmConnection::Anthropic => self.anthropic()?.find(),
        }
    }

    fn chat(&self, connection: &LlmConnection, chat: &LlmChat) -> anyhow::Result<LlmReply> {
        match connection {
            LlmConnection::Ollama(url) => self.ollama.chat(url, chat),
            LlmConnection::Anthropic => self.anthropic()?.chat(chat),
        }
    }
}

impl RoutingLlmServerRepository {
    /// The Anthropic adapter, or an unavailable provider without a key.
    fn anthropic(&self) -> anyhow::Result<&AnthropicLlmServerRepository> {
        self.anthropic.as_ref().ok_or_else(|| {
            anyhow::anyhow!("YARRTUBE_ANTHROPIC_API_KEY isn't set").context(LlmServerUnreachable)
        })
    }
}

/// An LLM provider answering with a fixed state, or unreachable, whatever
/// the connection.
#[cfg(test)]
pub struct FakeLlmServerRepository {
    server: Option<LlmServer>,
    /// Whether `find` fails as when nothing can connect, rather than as a
    /// server that connected but failed.
    unreachable: bool,
    /// The answers `chat` gives, in order; once used up, it fails.
    replies: std::sync::Mutex<std::collections::VecDeque<anyhow::Result<LlmReply>>>,
    chats: std::sync::Mutex<Vec<(LlmConnection, LlmChat)>>,
    on_chat: Option<Box<dyn Fn() + Send + Sync>>,
}

#[cfg(test)]
impl FakeLlmServerRepository {
    pub fn new(server: LlmServer) -> Self {
        Self::with(Some(server))
    }

    /// A server whose check fails, though not as a connect failure (see
    /// `unreachable()`): `find` fails with a plain error.
    pub fn failing() -> Self {
        Self::with(None)
    }

    /// A server nothing can connect to: `find` fails with
    /// `unreachable_error()`.
    pub fn unreachable() -> Self {
        Self {
            unreachable: true,
            ..Self::with(None)
        }
    }

    /// Answers `chat` with `replies`, in order.
    pub fn with_replies(self, replies: Vec<anyhow::Result<LlmReply>>) -> Self {
        Self {
            replies: std::sync::Mutex::new(replies.into()),
            ..self
        }
    }

    /// Runs `hook` while `chat` is in flight.
    pub fn with_on_chat(self, hook: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            on_chat: Some(Box::new(hook)),
            ..self
        }
    }

    /// The chats received, in order.
    pub fn chats(&self) -> Vec<LlmChat> {
        self.chats
            .lock()
            .unwrap()
            .iter()
            .map(|(_, chat)| chat.clone())
            .collect()
    }

    /// The connection each chat was sent to, in order.
    pub fn connections(&self) -> Vec<LlmConnection> {
        self.chats
            .lock()
            .unwrap()
            .iter()
            .map(|(connection, _)| connection.clone())
            .collect()
    }

    fn with(server: Option<LlmServer>) -> Self {
        Self {
            server,
            unreachable: false,
            replies: Default::default(),
            chats: Default::default(),
            on_chat: None,
        }
    }
}

/// What the adapter fails with when nothing can connect to the server.
#[cfg(test)]
pub fn unreachable_error() -> anyhow::Error {
    anyhow::anyhow!("connection refused").context(LlmServerUnreachable)
}

#[cfg(test)]
impl LlmServerRepository for FakeLlmServerRepository {
    fn find(&self, _connection: &LlmConnection) -> anyhow::Result<LlmServer> {
        match (&self.server, self.unreachable) {
            (Some(server), _) => Ok(server.clone()),
            (None, true) => Err(unreachable_error()),
            (None, false) => Err(anyhow::anyhow!("connection refused")),
        }
    }

    fn chat(&self, connection: &LlmConnection, chat: &LlmChat) -> anyhow::Result<LlmReply> {
        self.chats
            .lock()
            .unwrap()
            .push((connection.clone(), chat.clone()));
        if let Some(hook) = &self.on_chat {
            hook();
        }
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(anyhow::anyhow!("no reply left")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm_server::{Effort, MIN_CHAT_TIMEOUT, OllamaUrl};
    use crate::infrastructure::repositories::anthropic_llm_server_repository::{
        AnthropicApiKey, AnthropicConfig,
    };

    #[test]
    fn it_should_route_an_ollama_connection_to_ollama() {
        let mut ollama = mockito::Server::new();
        let chat_mock = ollama
            .mock("POST", "/api/chat")
            .with_body(r#"{"message": {"content": "Three games."}, "done_reason": "stop"}"#)
            .create();
        let repository = RoutingLlmServerRepository::new(OllamaLlmServerRepository::new(), None);

        let reply = repository.chat(
            &LlmConnection::Ollama(OllamaUrl::new(ollama.url()).unwrap()),
            &chat(),
        );

        assert_eq!(
            reply.map(|reply| reply.content).map_err(|e| e.to_string()),
            Ok("Three games.".to_string())
        );
        chat_mock.assert();
    }

    #[test]
    fn it_should_route_an_anthropic_connection_to_anthropic() {
        let mut anthropic = mockito::Server::new();
        let messages = anthropic
            .mock("POST", "/v1/messages")
            .with_body(
                r#"{"content": [{"type": "text", "text": "Three games."}], "stop_reason": "end_turn", "usage": {"input_tokens": 9, "output_tokens": 3}}"#,
            )
            .create();
        let repository = RoutingLlmServerRepository::new(
            OllamaLlmServerRepository::new(),
            Some(AnthropicLlmServerRepository::new(AnthropicConfig {
                api_url: anthropic.url(),
                api_key: AnthropicApiKey::new("sk-ant-test".to_string()),
            })),
        );

        let reply = repository.chat(&LlmConnection::Anthropic, &chat());

        assert_eq!(
            reply.map(|reply| reply.content).map_err(|e| e.to_string()),
            Ok("Three games.".to_string())
        );
        messages.assert();
    }

    #[test]
    fn it_should_report_anthropic_without_a_key_as_unreachable() {
        let repository = RoutingLlmServerRepository::new(OllamaLlmServerRepository::new(), None);

        let checked = repository
            .find(&LlmConnection::Anthropic)
            .map_err(|e| (e.is::<LlmServerUnreachable>(), format!("{e:#}")));
        let chatted = repository
            .chat(&LlmConnection::Anthropic, &chat())
            .map_err(|e| (e.is::<LlmServerUnreachable>(), format!("{e:#}")));

        let unavailable = Err((
            true,
            "the LLM provider is unavailable: YARRTUBE_ANTHROPIC_API_KEY isn't set".to_string(),
        ));
        assert_eq!(checked.map(|_| ()), unavailable);
        assert_eq!(chatted.map(|_| ()), unavailable);
    }

    fn chat() -> LlmChat {
        LlmChat {
            model: "claude-haiku-5-5".to_string(),
            system: "Be brief.".to_string(),
            user: "Title: Sicilian".to_string(),
            context_length: 12288,
            temperature: 0.3,
            thinking: false,
            effort: Effort::Medium,
            max_output_tokens: 1024,
            format: None,
            timeout: MIN_CHAT_TIMEOUT,
        }
    }
}
