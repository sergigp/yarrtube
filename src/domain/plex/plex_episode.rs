/// An episode as Plex lists it: one per media file, so an episode split
/// across several parts is listed once per file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexEpisode {
    pub rating_key: String,
    pub section_id: String,
    /// The file's server-side path.
    pub file: String,
}
