use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlaylistCreated {
    pub playlist_id: String,
}

impl PlaylistCreated {
    pub const EVENT_TYPE: &str = "playlist_created";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlaylistDeleted {
    pub playlist_id: String,
    pub name: String,
    pub path: String,
}

impl PlaylistDeleted {
    pub const EVENT_TYPE: &str = "playlist_deleted";
}
