use crate::domain::channel::Channel;
use crate::infrastructure::repositories::filesystem_channel_avatar_repository::ChannelAvatarRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::youtube_channel_repository::YoutubeChannelRepository;
use std::sync::Arc;
use tracing::{info, warn};

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
        if self.has_stored_avatar(&channel) {
            return channel;
        }
        let Some(avatar_filename) = self.download_avatar(&channel) else {
            return channel;
        };

        info!(channel_id = %channel.id, avatar = %avatar_filename, "restored missing channel avatar");
        self.record_avatar(channel, avatar_filename)
    }
}

impl ChannelAvatarRestorer {
    /// A file whose presence can't be checked counts as stored, so a
    /// transient filesystem error doesn't trigger a download every pass.
    fn has_stored_avatar(&self, channel: &Channel) -> bool {
        let Some(avatar_filename) = &channel.avatar_filename else {
            return false;
        };
        self.avatar_repository
            .exists(avatar_filename)
            .unwrap_or_else(|e| {
                warn!(channel_id = %channel.id, error = %e, "failed to check channel avatar file");
                true
            })
    }

    fn download_avatar(&self, channel: &Channel) -> Option<String> {
        let avatar_url = match self.lookup.resolve(&channel.id) {
            Ok(resolved) => resolved.and_then(|resolved| resolved.avatar_url)?,
            Err(e) => {
                warn!(channel_id = %channel.id, error = %e, "failed to look up channel avatar");
                return None;
            }
        };
        self.avatar_repository
            .store(&channel.id, &avatar_url)
            .unwrap_or_else(|e| {
                warn!(channel_id = %channel.id, error = %e, "failed to store channel avatar");
                None
            })
    }

    /// Only a filename different from the recorded one is saved, onto the
    /// channel as stored now rather than as read at the start of the pass, so
    /// a settings change made during the download isn't overwritten. A failed
    /// save still returns the updated channel: its file is on disk, and the
    /// next pass downloads it again and retries the save.
    fn record_avatar(&self, channel: Channel, avatar_filename: String) -> Channel {
        if channel.avatar_filename.as_deref() == Some(avatar_filename.as_str()) {
            return channel;
        }
        let channel = self
            .current_channel(channel)
            .with_avatar_filename(Some(avatar_filename));
        if let Err(e) = self.channel_repository.update(&channel) {
            warn!(channel_id = %channel.id, error = %e, "failed to record restored channel avatar");
        }
        channel
    }

    /// The channel as stored now, falling back to `channel` when it can't be
    /// read or no longer exists.
    fn current_channel(&self, channel: Channel) -> Channel {
        match self.channel_repository.find(&channel.id) {
            Ok(Some(current)) => current,
            Ok(None) => channel,
            Err(e) => {
                warn!(channel_id = %channel.id, error = %e, "failed to re-read channel before recording its avatar");
                channel
            }
        }
    }
}
