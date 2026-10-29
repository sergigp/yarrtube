/// A playlist as Plex lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexPlaylist {
    pub rating_key: String,
    pub title: String,
}
