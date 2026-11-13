use super::playback_position::PlaybackPosition;

/// A watch state reduced to what both yarrtube and Plex can represent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchProgress {
    Unplayed,
    InProgress(PlaybackPosition),
    Watched,
}
