use super::playback_position::PlaybackPosition;

/// A watch state reduced to what both yarrtube and Plex can represent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchProgress {
    Unplayed,
    InProgress(PlaybackPosition),
    Watched,
}

impl WatchProgress {
    pub fn is_in_progress(&self) -> bool {
        matches!(self, Self::InProgress(_))
    }
}
