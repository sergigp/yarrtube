/// A smart playlist yarrtube created for a category in a Plex library
/// section, keyed by section and label (ignoring case).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryPlaylist {
    pub section_id: String,
    /// The category's name, the label the playlist filters by.
    pub label: String,
    pub rating_key: String,
    pub title: String,
}
