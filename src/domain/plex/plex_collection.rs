/// A collection in the configured Plex library section, one per tracked
/// playlist/channel, keyed by its title (the playlist's/channel's name).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // constructed once the HTTP adapter parses responses
pub struct PlexCollection {
    pub rating_key: String,
    pub title: String,
}
