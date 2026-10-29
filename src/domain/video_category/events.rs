use serde::Serialize;

/// The category set of stored video records, or the name of a category in
/// it, changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoCategoriesChanged {
    /// Records whose category set, or a name in it, changed. Never empty.
    pub video_ids: Vec<String>,
}

impl VideoCategoriesChanged {
    pub const EVENT_TYPE: &str = "video_categories_changed";
}
