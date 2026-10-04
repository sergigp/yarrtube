use crate::domain::playlist::PlaylistId;
use crate::domain::video::VideoRecordId;
use chrono::{DateTime, Utc};

/// Records that a `Video` belongs to a `Playlist`, and its position in that
/// playlist. `id` is assigned by storage on insert; `0` before a fresh row
/// has been persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistVideo {
    pub id: i64,
    pub playlist_id: PlaylistId,
    pub video_id: VideoRecordId,
    pub position: i64,
    pub created_at: DateTime<Utc>,
}

impl PlaylistVideo {
    /// A video discovered in a playlist, recording its current position in
    /// that playlist.
    pub fn create(
        playlist_id: PlaylistId,
        video_id: VideoRecordId,
        position: i64,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: 0,
            playlist_id,
            video_id,
            position,
            created_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_build_a_playlist_video_with_a_known_position() {
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let playlist_video = PlaylistVideo::create(
            PlaylistId::new("PL1").unwrap(),
            VideoRecordId::new_generated(),
            3,
            now,
        );

        assert_eq!(playlist_video.position, 3);
    }
}
