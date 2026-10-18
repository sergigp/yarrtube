use crate::domain::channel::ChannelHandle;
#[cfg(test)]
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::shared::ytdlp;
use std::path::PathBuf;
#[cfg(test)]
use std::sync::Arc;

/// One of a channel's current videos, as discovered by a fresh listing —
/// `position` is the recency rank within that listing (`0` = most recent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelVideoListing {
    pub youtube_id: String,
    pub title: String,
    pub position: i64,
}

/// Discovers a channel's current videos, capped at a given limit, injected
/// into `ChannelVideoReconciler`.
pub trait ChannelVideosRepository: Send + Sync {
    fn list_current_videos(
        &self,
        channel_id: &ChannelHandle,
        limit: u32,
    ) -> anyhow::Result<Vec<ChannelVideoListing>>;
}

/// Backed by `yt-dlp` against the channel's `/videos` tab — see
/// design.md's "Channel video discovery via yt-dlp, not the YouTube Data
/// API" decision.
pub struct YtDlpChannelVideosRepository {
    ytdlp_path: PathBuf,
}

impl YtDlpChannelVideosRepository {
    pub fn new(ytdlp_path: PathBuf) -> Self {
        Self { ytdlp_path }
    }
}

impl ChannelVideosRepository for YtDlpChannelVideosRepository {
    fn list_current_videos(
        &self,
        channel_id: &ChannelHandle,
        limit: u32,
    ) -> anyhow::Result<Vec<ChannelVideoListing>> {
        let channel_url = format!("https://www.youtube.com/{}/videos", channel_id.as_str());
        let entries = ytdlp::list_channel_videos(&self.ytdlp_path, &channel_url, limit)?;
        Ok(entries
            .into_iter()
            .enumerate()
            .map(|(position, entry)| ChannelVideoListing {
                youtube_id: entry.video_id,
                title: entry.title,
                position: position as i64,
            })
            .collect())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeChannelVideosRepository {
    pub(crate) videos: std::sync::Mutex<Vec<ChannelVideoListing>>,
    pub(crate) fails: std::sync::Mutex<bool>,
    /// Deletes the listed channel from this repository, as if a user deleted
    /// it while the reconcile was running.
    pub(crate) deletes_from: Option<Arc<dyn ChannelRepository>>,
}

#[cfg(test)]
impl FakeChannelVideosRepository {
    pub fn with_videos(videos: Vec<ChannelVideoListing>) -> Self {
        Self {
            videos: std::sync::Mutex::new(videos),
            fails: std::sync::Mutex::new(false),
            deletes_from: None,
        }
    }

    pub fn failing() -> Self {
        Self {
            videos: std::sync::Mutex::new(Vec::new()),
            fails: std::sync::Mutex::new(true),
            deletes_from: None,
        }
    }

    pub fn deleting_the_channel_from(repository: Arc<dyn ChannelRepository>) -> Self {
        Self {
            deletes_from: Some(repository),
            ..Self::default()
        }
    }
}

#[cfg(test)]
impl ChannelVideosRepository for FakeChannelVideosRepository {
    fn list_current_videos(
        &self,
        channel_id: &ChannelHandle,
        limit: u32,
    ) -> anyhow::Result<Vec<ChannelVideoListing>> {
        if let Some(repository) = &self.deletes_from {
            repository.delete(channel_id)?;
        }
        if *self.fails.lock().unwrap() {
            anyhow::bail!("yt-dlp failed to list channel videos");
        }
        // Like yt-dlp's `-I 1:{limit}`: only the `limit` most recent uploads.
        Ok(self
            .videos
            .lock()
            .unwrap()
            .iter()
            .take(limit as usize)
            .cloned()
            .collect())
    }
}
