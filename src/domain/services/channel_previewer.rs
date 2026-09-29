use crate::domain::channel::{ChannelHandle, ChannelPreview, PreviewChannelError};
use crate::infrastructure::repositories::youtube_channel_repository::{
    ResolvedChannel, YoutubeChannelRepository,
};
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
        let resolved = self.resolve_on_youtube(&id)?;
        Ok(ChannelPreview {
            id,
            name: resolved.title,
            avatar_url: resolved.avatar_url,
        })
    }
}

impl ChannelPreviewer {
    fn resolve_on_youtube(
        &self,
        id: &ChannelHandle,
    ) -> Result<ResolvedChannel, PreviewChannelError> {
        match self.lookup.resolve(id) {
            Ok(Some(resolved)) => Ok(resolved),
            Ok(None) => Err(PreviewChannelError::YoutubeChannelNotFound(id.clone())),
            Err(e) => Err(PreviewChannelError::Lookup(e)),
        }
    }
}
