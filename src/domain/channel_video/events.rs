use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoAddedToChannel {
    pub channel_id: String,
    pub video_id: String,
}

impl VideoAddedToChannel {
    pub const EVENT_TYPE: &str = "video_added_to_channel";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoRemovedFromChannel {
    pub channel_id: String,
    pub video_id: String,
    pub title: String,
    pub filename: Option<String>,
    pub thumbnail_filename: Option<String>,
    pub was_downloaded: bool,
}

impl VideoRemovedFromChannel {
    pub const EVENT_TYPE: &str = "video_removed_from_channel";
}
