use crate::domain::channel::Channel;
use crate::infrastructure::repositories::filesystem_channel_avatar_repository::ChannelAvatarRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::youtube_channel_repository::YoutubeChannelRepository;
use std::sync::Arc;

/// Downloads a channel's avatar again when its file is missing (e.g. lost
/// with the container-local avatars directory older images used), so the
/// channel's show poster can be written again.
#[derive(Clone)]
pub struct ChannelAvatarRestorer {
    channel_repository: Arc<dyn ChannelRepository>,
    lookup: Arc<dyn YoutubeChannelRepository>,
    avatar_repository: Arc<dyn ChannelAvatarRepository>,
}

impl ChannelAvatarRestorer {
    pub fn new(
        channel_repository: Arc<dyn ChannelRepository>,
        lookup: Arc<dyn YoutubeChannelRepository>,
        avatar_repository: Arc<dyn ChannelAvatarRepository>,
    ) -> Self {
        Self {
            channel_repository,
            lookup,
            avatar_repository,
        }
    }
}

pub trait ChannelAvatarRestorerApi: Send + Sync {
    /// Returns `channel` as is when its recorded avatar file exists.
    /// Otherwise fetches the channel's current avatar from YouTube, stores
    /// it and records its filename on the channel, returning the updated
    /// channel. Best effort: a failure is logged and returns `channel`
    /// unchanged, so the next reconcile pass tries again.
    fn restore_missing(&self, channel: Channel) -> Channel;
}

impl ChannelAvatarRestorerApi for ChannelAvatarRestorer {
    fn restore_missing(&self, channel: Channel) -> Channel {
        self.download_avatar(&channel);
        channel
    }
}

impl ChannelAvatarRestorer {
    fn download_avatar(&self, channel: &Channel) -> Option<String> {
        let avatar_url = self.lookup.resolve(&channel.id).ok()??.avatar_url?;
        self.avatar_repository
            .store(&channel.id, &avatar_url)
            .ok()?
    }
}
