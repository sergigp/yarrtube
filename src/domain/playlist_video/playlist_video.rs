use crate::domain::playlist::PlaylistId;
use crate::domain::video::VideoId;
use chrono::{DateTime, Utc};

/// Records that a `Video` belongs to a `Playlist`, and its position in that
/// playlist. `id` is assigned by storage on insert; `0` before a fresh row
/// has been persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistVideo {
    pub id: i64,
    pub playlist_id: PlaylistId,
    pub video_id: VideoId,
    pub position: i64,
    pub created_at: DateTime<Utc>,
    /// A favorite no longer in the YouTube playlist, kept anyway.
    pub kept: bool,
}

impl PlaylistVideo {
    /// A video discovered in a playlist, recording its current position in
    /// that playlist.
    pub fn create(
        playlist_id: PlaylistId,
        video_id: VideoId,
        position: i64,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: 0,
            playlist_id,
            video_id,
            position,
            created_at: now,
            kept: false,
        }
    }

    /// Kept at `position`, after the listed items.
    pub fn keep(self, position: i64, now: DateTime<Utc>) -> Self {
        Self {
            position,
            kept: true,
            created_at: now,
            ..self
        }
    }

    /// Listed again at `position`, no longer kept.
    pub fn listed_at(self, position: i64, now: DateTime<Utc>) -> Self {
        Self {
            position,
            created_at: now,
            kept: false,
            ..self
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
            VideoId::new_generated(),
            3,
            now,
        );

        assert_eq!(playlist_video.position, 3);
    }
    #[test]
    fn it_should_keep_a_playlist_video_at_a_new_position() {
        let created_at = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let playlist_video = PlaylistVideo {
            id: 7,
            ..PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                VideoId::new_generated(),
                2,
                created_at,
            )
        };

        let kept = playlist_video.clone().keep(5, now);

        assert_eq!(
            kept,
            PlaylistVideo {
                position: 5,
                created_at: now,
                kept: true,
                ..playlist_video
            }
        );
    }
}
