/// The executor lane a task runs in. Each lane has its own concurrency cap;
/// `Exclusive` runs alone, with nothing else running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskLane {
    Download,
    Thumbnail,
    Subtitles,
    VideoSync,
    Light,
    Plex,
    Summaries,
    Exclusive,
}

impl TaskLane {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Download => "download",
            Self::Thumbnail => "thumbnail",
            Self::Subtitles => "subtitles",
            Self::VideoSync => "video_sync",
            Self::Light => "light",
            Self::Plex => "plex",
            Self::Summaries => "summaries",
            Self::Exclusive => "exclusive",
        }
    }
}
