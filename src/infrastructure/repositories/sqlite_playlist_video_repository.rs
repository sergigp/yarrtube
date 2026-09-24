use crate::domain::playlist_video::PlaylistVideo;
use crate::domain::shared::{PlaylistId, VideoId, VideoRecordId};
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::Mutex;

pub trait PlaylistVideoRepository: Send + Sync {
    /// Insert-or-replace keyed by `(playlist_id, video_id)`.
    fn save(&self, playlist_video: &PlaylistVideo) -> anyhow::Result<()>;
    fn find_by_youtube_video(
        &self,
        playlist_id: &PlaylistId,
        youtube_video_id: &VideoId,
    ) -> anyhow::Result<Option<PlaylistVideo>>;
    /// Finds whichever playlist a video belongs to, keyed by the video's own
    /// surrogate ID rather than a `(playlist_id, youtube_video_id)` pair.
    fn find_by_video(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<PlaylistVideo>>;
    /// Ordered by position (YouTube-defined order), with no-position rows
    /// sorted last, by insertion order.
    fn list_for_playlist(&self, playlist_id: &PlaylistId) -> anyhow::Result<Vec<PlaylistVideo>>;
    fn delete(&self, playlist_id: &PlaylistId, youtube_video_id: &VideoId) -> anyhow::Result<()>;
    fn delete_all_for_playlist(&self, playlist_id: &PlaylistId) -> anyhow::Result<()>;
}

pub struct SqlitePlaylistVideoRepository {
    conn: Mutex<Connection>,
}

impl SqlitePlaylistVideoRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }
}

impl PlaylistVideoRepository for SqlitePlaylistVideoRepository {
    fn save(&self, playlist_video: &PlaylistVideo) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| {
                tracing::error!(playlist_id = %playlist_video.playlist_id, "database lock poisoned")
            })
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "INSERT INTO playlist_videos (playlist_id, video_id, position, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (playlist_id, video_id) DO UPDATE SET
                position = excluded.position,
                created_at = excluded.created_at",
            params![
                playlist_video.playlist_id.as_str(),
                playlist_video.video_id.as_str(),
                playlist_video.position,
                playlist_video.created_at.to_rfc3339(),
            ],
        )
        .inspect_err(|e| {
            tracing::error!(playlist_id = %playlist_video.playlist_id, error = %e, "failed to save playlist video")
        })
        .context("failed to save playlist video")?;
        Ok(())
    }

    fn find_by_youtube_video(
        &self,
        playlist_id: &PlaylistId,
        youtube_video_id: &VideoId,
    ) -> anyhow::Result<Option<PlaylistVideo>> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(playlist_id = %playlist_id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.query_row(
            "SELECT pv.id, pv.playlist_id, pv.video_id, pv.position, pv.created_at
             FROM playlist_videos pv
             JOIN videos v ON v.id = pv.video_id
             WHERE pv.playlist_id = ?1 AND v.youtube_id = ?2",
            params![playlist_id.as_str(), youtube_video_id.as_str()],
            row_to_columns,
        )
        .optional()
        .inspect_err(|e| {
            tracing::error!(playlist_id = %playlist_id, error = %e, "failed to find playlist video")
        })
        .context("failed to find playlist video")?
        .map(columns_to_playlist_video)
        .transpose()
    }

    fn find_by_video(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<PlaylistVideo>> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(video_id = %video_id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.query_row(
            "SELECT id, playlist_id, video_id, position, created_at
             FROM playlist_videos WHERE video_id = ?1",
            params![video_id.as_str()],
            row_to_columns,
        )
        .optional()
        .inspect_err(|e| {
            tracing::error!(video_id = %video_id, error = %e, "failed to find playlist video by video")
        })
        .context("failed to find playlist video by video")?
        .map(columns_to_playlist_video)
        .transpose()
    }

    fn list_for_playlist(&self, playlist_id: &PlaylistId) -> anyhow::Result<Vec<PlaylistVideo>> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(playlist_id = %playlist_id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, playlist_id, video_id, position, created_at
                 FROM playlist_videos WHERE playlist_id = ?1
                 ORDER BY position IS NULL, position ASC, id ASC",
            )
            .inspect_err(|e| {
                tracing::error!(playlist_id = %playlist_id, error = %e, "failed to prepare list-playlist-videos query")
            })
            .context("failed to prepare list-playlist-videos query")?;
        let rows = stmt
            .query_map(params![playlist_id.as_str()], row_to_columns)
            .inspect_err(|e| {
                tracing::error!(playlist_id = %playlist_id, error = %e, "failed to list playlist videos")
            })
            .context("failed to list playlist videos")?;

        rows.map(|row| {
            let columns = row
                .inspect_err(|e| tracing::error!(error = %e, "failed to read playlist video row"))
                .context("failed to read playlist video row")?;
            columns_to_playlist_video(columns)
        })
        .collect()
    }

    fn delete(&self, playlist_id: &PlaylistId, youtube_video_id: &VideoId) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(playlist_id = %playlist_id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "DELETE FROM playlist_videos
             WHERE playlist_id = ?1 AND video_id IN (
                SELECT id FROM videos WHERE youtube_id = ?2
             )",
            params![playlist_id.as_str(), youtube_video_id.as_str()],
        )
        .inspect_err(|e| {
            tracing::error!(playlist_id = %playlist_id, error = %e, "failed to delete playlist video")
        })
        .context("failed to delete playlist video")?;
        Ok(())
    }

    fn delete_all_for_playlist(&self, playlist_id: &PlaylistId) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(playlist_id = %playlist_id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "DELETE FROM playlist_videos WHERE playlist_id = ?1",
            params![playlist_id.as_str()],
        )
        .inspect_err(|e| {
            tracing::error!(playlist_id = %playlist_id, error = %e, "failed to delete playlist videos for playlist")
        })
        .context("failed to delete playlist videos for playlist")?;
        Ok(())
    }
}

type Columns = (i64, String, String, Option<i64>, String);

fn row_to_columns(row: &rusqlite::Row) -> rusqlite::Result<Columns> {
    Ok((
        row.get::<_, i64>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, Option<i64>>(3)?,
        row.get::<_, String>(4)?,
    ))
}

fn columns_to_playlist_video(columns: Columns) -> anyhow::Result<PlaylistVideo> {
    let (id, playlist_id, video_id, position, created_at) = columns;
    Ok(PlaylistVideo {
        id,
        playlist_id: PlaylistId::new(playlist_id)?,
        video_id: VideoRecordId::new(video_id)?,
        position,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .context("failed to parse stored created_at")?
            .with_timezone(&Utc),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::video::Video;

    fn repo() -> SqlitePlaylistVideoRepository {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::shared::sqlite_migrations::apply(&mut conn).unwrap();
        SqlitePlaylistVideoRepository::new(conn)
    }

    fn playlist_id() -> PlaylistId {
        PlaylistId::new("PL1").unwrap()
    }

    /// Inserts a `videos` row directly (this repository's tests don't go
    /// through `SqliteVideoRepository`) and returns the `Video` it
    /// represents, so `video_id`/`youtube_id` stay consistent.
    fn seed_video(repo: &SqlitePlaylistVideoRepository, youtube_id: &str, title: &str) -> Video {
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let video = Video::create(VideoId::new(youtube_id).unwrap(), title, now);
        let conn = repo.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO videos (id, youtube_id, title, status, quality, filename, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'PENDING', NULL, NULL, ?4, ?4)",
            params![
                video.id.as_str(),
                video.youtube_id.as_str(),
                video.title,
                video.created_at.to_rfc3339(),
            ],
        )
        .unwrap();
        video
    }

    #[test]
    fn it_should_return_the_playlist_video_after_saving_it() {
        let repo = repo();
        let video = seed_video(&repo, "yt1", "First");
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        repo.save(&PlaylistVideo::create_with_position(
            playlist_id(),
            video.id.clone(),
            0,
            now,
        ))
        .unwrap();

        let found = repo
            .find_by_youtube_video(&playlist_id(), &VideoId::new("yt1").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(found.video_id, video.id);
        assert_eq!(found.position, Some(0));
    }

    #[test]
    fn it_should_round_trip_a_playlist_video_with_no_position() {
        let repo = repo();
        let video = seed_video(&repo, "yt1", "First");
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        repo.save(&PlaylistVideo::create(playlist_id(), video.id.clone(), now))
            .unwrap();

        let found = repo
            .find_by_youtube_video(&playlist_id(), &VideoId::new("yt1").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(found.position, None);
    }

    #[test]
    fn it_should_list_playlist_videos_ordered_by_position() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let c = seed_video(&repo, "yt_c", "Third");
        let a = seed_video(&repo, "yt_a", "First");
        let b = seed_video(&repo, "yt_b", "Second");
        repo.save(&PlaylistVideo::create_with_position(
            playlist_id(),
            c.id.clone(),
            2,
            now,
        ))
        .unwrap();
        repo.save(&PlaylistVideo::create_with_position(
            playlist_id(),
            a.id.clone(),
            0,
            now,
        ))
        .unwrap();
        repo.save(&PlaylistVideo::create_with_position(
            playlist_id(),
            b.id.clone(),
            1,
            now,
        ))
        .unwrap();

        let videos = repo.list_for_playlist(&playlist_id()).unwrap();

        assert_eq!(
            videos
                .iter()
                .map(|pv| pv.video_id.clone())
                .collect::<Vec<_>>(),
            vec![a.id, b.id, c.id]
        );
    }

    #[test]
    fn it_should_sort_playlist_videos_with_no_position_last() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let no_position = seed_video(&repo, "yt_no_position", "No position");
        let positioned = seed_video(&repo, "yt_positioned", "Positioned");
        repo.save(&PlaylistVideo::create(
            playlist_id(),
            no_position.id.clone(),
            now,
        ))
        .unwrap();
        repo.save(&PlaylistVideo::create_with_position(
            playlist_id(),
            positioned.id.clone(),
            5,
            now,
        ))
        .unwrap();

        let videos = repo.list_for_playlist(&playlist_id()).unwrap();

        assert_eq!(
            videos
                .iter()
                .map(|pv| pv.video_id.clone())
                .collect::<Vec<_>>(),
            vec![positioned.id, no_position.id]
        );
    }

    #[test]
    fn it_should_delete_only_the_named_playlist_video() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let one = seed_video(&repo, "yt1", "One");
        let two = seed_video(&repo, "yt2", "Two");
        repo.save(&PlaylistVideo::create(playlist_id(), one.id.clone(), now))
            .unwrap();
        repo.save(&PlaylistVideo::create(playlist_id(), two.id.clone(), now))
            .unwrap();

        repo.delete(&playlist_id(), &VideoId::new("yt1").unwrap())
            .unwrap();

        assert!(
            repo.find_by_youtube_video(&playlist_id(), &VideoId::new("yt1").unwrap())
                .unwrap()
                .is_none()
        );
        assert!(
            repo.find_by_youtube_video(&playlist_id(), &VideoId::new("yt2").unwrap())
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn it_should_delete_every_playlist_video_for_the_playlist_and_leave_others_untouched() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let one = seed_video(&repo, "yt1", "One");
        let two = seed_video(&repo, "yt2", "Two");
        repo.save(&PlaylistVideo::create(playlist_id(), one.id.clone(), now))
            .unwrap();
        let other_playlist_id = PlaylistId::new("PL2").unwrap();
        repo.save(&PlaylistVideo::create(
            other_playlist_id.clone(),
            two.id.clone(),
            now,
        ))
        .unwrap();

        repo.delete_all_for_playlist(&playlist_id()).unwrap();

        assert!(repo.list_for_playlist(&playlist_id()).unwrap().is_empty());
        assert_eq!(repo.list_for_playlist(&other_playlist_id).unwrap().len(), 1);
    }

    #[test]
    fn it_should_return_none_when_finding_a_missing_playlist_video() {
        let repo = repo();

        let found = repo
            .find_by_youtube_video(&playlist_id(), &VideoId::new("yt1").unwrap())
            .unwrap();

        assert!(found.is_none());
    }

    #[test]
    fn it_should_find_a_playlist_video_by_its_video_record_id() {
        let repo = repo();
        let video = seed_video(&repo, "yt1", "First");
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        repo.save(&PlaylistVideo::create(playlist_id(), video.id.clone(), now))
            .unwrap();

        let found = repo.find_by_video(&video.id).unwrap().unwrap();

        assert_eq!(found.playlist_id, playlist_id());
        assert_eq!(found.video_id, video.id);
    }

    #[test]
    fn it_should_return_none_when_finding_by_video_for_an_untracked_video() {
        let repo = repo();

        let found = repo
            .find_by_video(&VideoRecordId::new("missing").unwrap())
            .unwrap();

        assert!(found.is_none());
    }
}
