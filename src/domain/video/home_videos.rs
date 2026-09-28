use super::home_video_view::HomeVideoView;

/// The videos of the three home sections, in section order. A YouTube video
/// appears in at most one of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeVideos {
    pub continue_watching: Vec<HomeVideoView>,
    pub quick_watches: Vec<HomeVideoView>,
    pub latest: Vec<HomeVideoView>,
}
