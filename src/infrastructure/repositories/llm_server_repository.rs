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
    #[expect(
        dead_code,
        reason = "walking skeleton: asserted by the Anthropic task tests"
    )]
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
