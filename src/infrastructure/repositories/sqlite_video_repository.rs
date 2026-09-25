use crate::domain::shared::Quality;
use crate::domain::video::{PlaybackPosition, Video, VideoStatus};
use crate::domain::video::{VideoId, VideoRecordId};
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params};
use std::sync::Mutex;

const VIDEO_COLUMNS: &str = "id, youtube_id, title, status, quality, filename, thumbnail_filename, duration_seconds, created_at, updated_at, watched_at, playback_position_seconds";

/// A `videos` row as read, before its values are parsed into a `Video`.
struct VideoRow {
    id: String,
    youtube_id: String,
    title: String,
    status: String,
    quality: Option<String>,
    filename: Option<String>,
    thumbnail_filename: Option<String>,
    duration_seconds: Option<i64>,
    created_at: String,
    updated_at: String,
    watched_at: Option<String>,
    playback_position_seconds: i64,
}

pub trait VideoRepository: Send + Sync {
    /// Plain insert-or-full-replace of every column — no conflict-merge
    /// policy. Callers decide what `Video` value to persist.
    fn save(&self, video: &Video) -> anyhow::Result<()>;
    fn find(&self, id: &VideoRecordId) -> anyhow::Result<Option<Video>>;
    /// Writes every mutable column, including `status` — unlike `save`,
    /// which is a plain insert-or-replace, `update` only touches a row that
    /// still exists.
    fn update(&self, video: &Video) -> anyhow::Result<()>;
    fn delete(&self, id: &VideoRecordId) -> anyhow::Result<()>;
    /// Every stored copy of a YouTube video, across all playlists/channels.
    fn find_by_youtube_id(&self, youtube_id: &VideoId) -> anyhow::Result<Vec<Video>>;
}

pub struct SqliteVideoRepository {
    conn: Mutex<Connection>,
}

impl SqliteVideoRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    /// Every stored video, for tests asserting the table's whole final state;
    /// the port itself has no use case that lists all videos.
    #[cfg(test)]
    pub fn list(&self) -> anyhow::Result<Vec<Video>> {
        let ids: Vec<String> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id FROM videos ORDER BY rowid ASC")?;
            stmt.query_map([], |row| row.get(0))?
                .collect::<Result<_, _>>()?
        };
        ids.into_iter()
            .map(|id| Ok(self.find(&VideoRecordId::new(id)?)?.unwrap()))
            .collect()
    }
}

impl VideoRepository for SqliteVideoRepository {
    fn save(&self, video: &Video) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(video_id = %video.id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "INSERT INTO videos (id, youtube_id, title, status, quality, filename, thumbnail_filename, duration_seconds, created_at, updated_at, watched_at, playback_position_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT (id) DO UPDATE SET
                youtube_id = excluded.youtube_id,
                title = excluded.title,
                status = excluded.status,
                quality = excluded.quality,
                filename = excluded.filename,
                thumbnail_filename = excluded.thumbnail_filename,
                duration_seconds = excluded.duration_seconds,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at,
                watched_at = excluded.watched_at,
                playback_position_seconds = excluded.playback_position_seconds",
            params![
                video.id.as_str(),
                video.youtube_id.as_str(),
                video.title,
                video.status.as_str(),
                video.quality.map(|q| q.as_str()),
                video.filename,
                video.thumbnail_filename,
                video.duration_seconds,
                video.created_at.to_rfc3339(),
                video.updated_at.to_rfc3339(),
                video.watched_at.map(|w| w.to_rfc3339()),
                video.playback_position.seconds(),
            ],
        )
        .inspect_err(|e| {
            tracing::error!(video_id = %video.id, error = %e, "failed to save video")
        })
        .context("failed to save video")?;
        Ok(())
    }

    fn find(&self, id: &VideoRecordId) -> anyhow::Result<Option<Video>> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(video_id = %id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.query_row(
            &format!("SELECT {VIDEO_COLUMNS} FROM videos WHERE id = ?1"),
            params![id.as_str()],
            Self::read_row,
        )
        .optional()
        .inspect_err(|e| tracing::error!(video_id = %id, error = %e, "failed to find video"))
        .context("failed to find video")?
        .map(Self::row_to_video)
        .transpose()
    }

    fn update(&self, video: &Video) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(video_id = %video.id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "UPDATE videos SET youtube_id = ?2, title = ?3, status = ?4, quality = ?5, filename = ?6, thumbnail_filename = ?7, duration_seconds = ?8, updated_at = ?9, watched_at = ?10, playback_position_seconds = ?11
             WHERE id = ?1",
            params![
                video.id.as_str(),
                video.youtube_id.as_str(),
                video.title,
                video.status.as_str(),
                video.quality.map(|q| q.as_str()),
                video.filename,
                video.thumbnail_filename,
                video.duration_seconds,
                video.updated_at.to_rfc3339(),
                video.watched_at.map(|w| w.to_rfc3339()),
                video.playback_position.seconds(),
            ],
        )
        .inspect_err(|e| {
            tracing::error!(video_id = %video.id, error = %e, "failed to update video")
        })
        .context("failed to update video")?;
        Ok(())
    }

    fn delete(&self, id: &VideoRecordId) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(video_id = %id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute("DELETE FROM videos WHERE id = ?1", params![id.as_str()])
            .inspect_err(|e| tracing::error!(video_id = %id, error = %e, "failed to delete video"))
            .context("failed to delete video")?;
        Ok(())
    }

    fn find_by_youtube_id(&self, youtube_id: &VideoId) -> anyhow::Result<Vec<Video>> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(youtube_id = %youtube_id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {VIDEO_COLUMNS} FROM videos WHERE youtube_id = ?1 ORDER BY rowid ASC"
            ))
            .context("failed to prepare find videos by youtube id")?;
        stmt.query_map(params![youtube_id.as_str()], Self::read_row)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .inspect_err(|e| {
                tracing::error!(youtube_id = %youtube_id, error = %e, "failed to find videos by youtube id")
            })
            .context("failed to find videos by youtube id")?
            .into_iter()
            .map(Self::row_to_video)
            .collect()
    }
}

impl SqliteVideoRepository {
    fn read_row(row: &Row<'_>) -> rusqlite::Result<VideoRow> {
        Ok(VideoRow {
            id: row.get(0)?,
            youtube_id: row.get(1)?,
            title: row.get(2)?,
            status: row.get(3)?,
            quality: row.get(4)?,
            filename: row.get(5)?,
            thumbnail_filename: row.get(6)?,
            duration_seconds: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            watched_at: row.get(10)?,
            playback_position_seconds: row.get(11)?,
        })
    }

    fn row_to_video(row: VideoRow) -> anyhow::Result<Video> {
        Ok(Video {
            id: VideoRecordId::new(row.id)?,
            youtube_id: VideoId::new(row.youtube_id)?,
            title: row.title,
            status: VideoStatus::parse(&row.status)?,
            quality: row.quality.map(Quality::new).transpose()?,
            filename: row.filename,
            thumbnail_filename: row.thumbnail_filename,
            duration_seconds: row.duration_seconds,
            created_at: Self::parse_timestamp(&row.created_at, "created_at")?,
            updated_at: Self::parse_timestamp(&row.updated_at, "updated_at")?,
            watched_at: row
                .watched_at
                .map(|w| Self::parse_timestamp(&w, "watched_at"))
                .transpose()?,
            playback_position: PlaybackPosition::new(row.playback_position_seconds)?,
        })
    }

    fn parse_timestamp(value: &str, column: &str) -> anyhow::Result<DateTime<Utc>> {
        Ok(DateTime::parse_from_rfc3339(value)
            .with_context(|| format!("failed to parse stored {column}"))?
            .with_timezone(&Utc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> SqliteVideoRepository {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::shared::sqlite_migrations::apply(&mut conn).unwrap();
        SqliteVideoRepository::new(conn)
    }

    fn video(title: &str, now: DateTime<Utc>) -> Video {
        Video::create(VideoId::new("yt1").unwrap(), title, now)
    }

    #[test]
    fn it_should_return_the_video_after_saving_it() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let video = video("First", now);

        repo.save(&video).unwrap();

        let found = repo.find(&video.id).unwrap().unwrap();
        assert_eq!(found.title, "First");
        assert_eq!(found.youtube_id.as_str(), "yt1");
        assert_eq!(found.status, VideoStatus::Pending);
        assert_eq!(found.quality, None);
    }

    #[test]
    fn it_should_allow_two_videos_to_share_the_same_youtube_id() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let a = Video::create(VideoId::new("shared").unwrap(), "A", now);
        let b = Video::create(VideoId::new("shared").unwrap(), "B", now);

        repo.save(&a).unwrap();
        repo.save(&b).unwrap();

        assert_eq!(repo.find(&a.id).unwrap().unwrap().title, "A");
        assert_eq!(repo.find(&b.id).unwrap().unwrap().title, "B");
    }

    #[test]
    fn it_should_round_trip_a_video_with_a_recorded_quality() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let downloaded = video("First", now).start_download(now).mark_downloaded(
            Quality::Mid,
            "First.mp4",
            None,
            None,
            now,
        );
        repo.save(&downloaded).unwrap();

        let found = repo.find(&downloaded.id).unwrap().unwrap();

        assert_eq!(found.quality, Some(Quality::Mid));
        assert_eq!(found.filename, Some("First.mp4".to_string()));
    }

    #[test]
    fn it_should_round_trip_a_video_with_a_recorded_thumbnail_filename() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let downloaded = video("First", now).start_download(now).mark_downloaded(
            Quality::Mid,
            "First.mp4",
            Some("First.jpg".to_string()),
            None,
            now,
        );
        repo.save(&downloaded).unwrap();

        let found = repo.find(&downloaded.id).unwrap().unwrap();

        assert_eq!(found.thumbnail_filename, Some("First.jpg".to_string()));
    }

    #[test]
    fn it_should_round_trip_a_video_with_a_recorded_duration() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let downloaded = video("First", now).start_download(now).mark_downloaded(
            Quality::Mid,
            "First.mp4",
            None,
            Some(223),
            now,
        );
        repo.save(&downloaded).unwrap();

        let found = repo.find(&downloaded.id).unwrap().unwrap();

        assert_eq!(found.duration_seconds, Some(223));
    }

    #[test]
    fn it_should_round_trip_a_video_with_no_recorded_duration() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let downloaded = video("First", now).start_download(now).mark_downloaded(
            Quality::Mid,
            "First.mp4",
            None,
            None,
            now,
        );
        repo.save(&downloaded).unwrap();

        let found = repo.find(&downloaded.id).unwrap().unwrap();

        assert_eq!(found.duration_seconds, None);
    }

    #[test]
    fn it_should_overwrite_every_field_including_status_when_saving_an_existing_video() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let original = video("First", now);
        repo.save(&original).unwrap();

        let later = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let updated = Video {
            title: "Renamed".to_string(),
            updated_at: later,
            ..original.clone()
        }
        .start_download(later);
        repo.save(&updated).unwrap();

        let found = repo.find(&original.id).unwrap().unwrap();
        assert_eq!(found.title, "Renamed");
        assert_eq!(found.status, VideoStatus::InProgress);
        assert_eq!(found.updated_at, later);
    }

    #[test]
    fn it_should_delete_only_the_named_video() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let one = video("One", now);
        let two = video("Two", now);
        repo.save(&one).unwrap();
        repo.save(&two).unwrap();

        repo.delete(&one.id).unwrap();

        assert!(repo.find(&one.id).unwrap().is_none());
        assert!(repo.find(&two.id).unwrap().is_some());
    }

    #[test]
    fn it_should_succeed_when_deleting_a_missing_video() {
        let repo = repo();

        assert!(repo.delete(&VideoRecordId::new_generated()).is_ok());
    }

    #[test]
    fn it_should_return_none_when_finding_a_missing_video() {
        let repo = repo();

        let found = repo.find(&VideoRecordId::new_generated()).unwrap();

        assert!(found.is_none());
    }

    #[test]
    fn it_should_return_the_stored_video_with_its_updated_status_after_update() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let original = video("First", now);
        repo.save(&original).unwrap();
        let stored = repo.find(&original.id).unwrap().unwrap();

        let later = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        repo.update(&stored.start_download(later)).unwrap();

        let found = repo.find(&original.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::InProgress);
        assert_eq!(found.updated_at, later);
    }
}
