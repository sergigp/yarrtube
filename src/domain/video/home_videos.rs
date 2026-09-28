use super::recent_video::RecentVideo;

/// The videos of the three home sections, in section order. A YouTube video
/// appears in at most one of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeVideos {
    pub continue_watching: Vec<RecentVideo>,
    pub quick_watches: Vec<RecentVideo>,
    pub latest: Vec<RecentVideo>,
}
