use crate::domain::playlist::PlaylistId;
use crate::domain::video::VideoRecordId;
use chrono::{DateTime, Utc};

/// Records that a `Video` belongs to a `Playlist`, and (for a YouTube-linked
/// playlist) its position in that playlist. `id` is assigned by storage on
/// insert; `0` before a fresh row has been persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistVideo {
    pub id: i64,
    pub playlist_id: PlaylistId,
    pub video_id: VideoRecordId,
    pub position: Option<i64>,
    pub created_at: DateTime<Utc>,
}

impl PlaylistVideo {
    /// A video added without a YouTube-defined position.
    pub fn create(playlist_id: PlaylistId, video_id: VideoRecordId, now: DateTime<Utc>) -> Self {
        Self {
            id: 0,
            playlist_id,
            video_id,
            position: None,
            created_at: now,
        }
    }

    /// A video discovered in a YouTube-linked playlist, recording its
    /// current position in that playlist.
    pub fn create_with_position(
        playlist_id: PlaylistId,
        video_id: VideoRecordId,
        position: i64,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            position: Some(position),
            ..Self::create(playlist_id, video_id, now)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_build_a_playlist_video_with_no_position() {
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let playlist_video = PlaylistVideo::create(
            PlaylistId::new("PL1").unwrap(),
            VideoRecordId::new_generated(),
            now,
        );

        assert_eq!(playlist_video.position, None);
        assert_eq!(playlist_video.created_at, now);
    }

    #[test]
    fn it_should_build_a_playlist_video_with_a_known_position() {
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let playlist_video = PlaylistVideo::create_with_position(
            PlaylistId::new("PL1").unwrap(),
            VideoRecordId::new_generated(),
            3,
            now,
        );

        assert_eq!(playlist_video.position, Some(3));
    }
}
