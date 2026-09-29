use super::channel_handle::ChannelHandle;

/// What YouTube reports about a channel before it is tracked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelPreview {
    pub id: ChannelHandle,
    pub name: String,
    pub avatar_url: Option<String>,
}
