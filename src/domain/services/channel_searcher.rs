use crate::domain::channel::Channel;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use std::sync::Arc;

/// Reads channels.
#[derive(Clone)]
pub struct ChannelSearcher {
    repository: Arc<dyn ChannelRepository>,
}

impl ChannelSearcher {
    pub fn new(repository: Arc<dyn ChannelRepository>) -> Self {
        Self { repository }
    }
}

pub trait ChannelSearcherApi: Send + Sync {
    fn search_all(&self) -> anyhow::Result<Vec<Channel>>;
}

impl ChannelSearcherApi for ChannelSearcher {
    fn search_all(&self) -> anyhow::Result<Vec<Channel>> {
        self.repository.list()
    }
}
