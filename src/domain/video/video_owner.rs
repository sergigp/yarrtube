use crate::domain::category::CategoryMember;
use crate::domain::video::VideoTarget;

/// The playlist or channel holding a video, as its sync needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoOwner {
    pub source: CategoryMember,
    pub target: VideoTarget,
    /// The video's playlist position; `None` for a channel video, which
    /// sorts by publish date.
    pub sort_position: Option<i64>,
    /// Whether the source opted in to AI summaries.
    pub summaries_enabled: bool,
    /// Whether the source opted in to AI categorization.
    pub categorization_enabled: bool,
    /// The channel's or playlist's name.
    pub source_name: String,
}

impl VideoOwner {
    /// Whether the source categorizes its videos: summaries and
    /// categorization both on, since a video is judged from its summary.
    pub fn categorizes(&self) -> bool {
        self.summaries_enabled && self.categorization_enabled
    }
}
