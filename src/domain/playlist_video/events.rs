use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoAddedToPlaylist {
    pub playlist_id: String,
    pub video_id: String,
}

impl VideoAddedToPlaylist {
    pub const EVENT_TYPE: &str = "video_added_to_playlist";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoRemovedFromPlaylist {
    pub playlist_id: String,
    pub video_id: String,
    pub title: String,
    pub filename: Option<String>,
    pub thumbnail_filename: Option<String>,
    pub was_downloaded: bool,
}

impl VideoRemovedFromPlaylist {
    pub const EVENT_TYPE: &str = "video_removed_from_playlist";
}
