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
}
