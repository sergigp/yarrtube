use crate::domain::channel::{ChannelHandle, ChannelPreview, PreviewChannelError};
use crate::infrastructure::repositories::youtube_channel_repository::YoutubeChannelRepository;
use std::sync::Arc;

/// Looks a YouTube channel up without tracking it or storing its avatar.
#[derive(Clone)]
pub struct ChannelPreviewer {
    lookup: Arc<dyn YoutubeChannelRepository>,
}

impl ChannelPreviewer {
    pub fn new(lookup: Arc<dyn YoutubeChannelRepository>) -> Self {
        Self { lookup }
    }
}

pub trait ChannelPreviewerApi: Send + Sync {
    /// Returns the channel's handle, YouTube title and avatar URL, persisting
    /// nothing.
    fn preview(&self, id: ChannelHandle) -> Result<ChannelPreview, PreviewChannelError>;
}

impl ChannelPreviewerApi for ChannelPreviewer {
    fn preview(&self, id: ChannelHandle) -> Result<ChannelPreview, PreviewChannelError> {
        let _ = &self.lookup;
        Err(PreviewChannelError::YoutubeChannelNotFound(id))
    }
}
