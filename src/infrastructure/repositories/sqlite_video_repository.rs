use crate::domain::shared::{PlaylistId, Quality, VideoId};
use crate::domain::video::{Video, VideoStatus};
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::Mutex;

pub trait VideoRepository: Send + Sync {
    /// Plain insert-or-full-replace of every column — no conflict-merge
    /// policy. Callers decide what `Video` value to persist.
    fn save(&self, video: &Video) -> anyhow::Result<()>;
    fn find(&self, playlist_id: &PlaylistId, video_id: &VideoId) -> anyhow::Result<Option<Video>>;
    fn list_for_playlist(&self, playlist_id: &PlaylistId) -> anyhow::Result<Vec<Video>>;
    fn delete(&self, playlist_id: &PlaylistId, video_id: &VideoId) -> anyhow::Result<()>;
    /// Writes every mutable column, including `status` — unlike `save`,
    /// which is a plain insert-or-replace, `update` only touches a row that
    /// still exists.
    fn update(&self, video: &Video) -> anyhow::Result<()>;
}

pub struct SqliteVideoRepository {
    conn: Mutex<Connection>,
}

impl SqliteVideoRepository {
    pub fn new(conn: Connection) -> anyhow::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS videos (
                playlist_id TEXT NOT NULL,
                video_id TEXT NOT NULL,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                quality TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (playlist_id, video_id)
            )",
            [],
        )
        .context("failed to create videos table")?;
        match conn.execute("ALTER TABLE videos ADD COLUMN filename TEXT", []) {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(_, Some(message)))
                if message.contains("duplicate column name") => {}
            Err(e) => return Err(e).context("failed to add filename column to videos table"),
        }
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn row_to_video(
        playlist_id: String,
        video_id: String,
        title: String,
        status: String,
        quality: Option<String>,
        filename: Option<String>,
        created_at: String,
        updated_at: String,
    ) -> anyhow::Result<Video> {
        Ok(Video {
            playlist_id: PlaylistId::new(playlist_id)?,
            video_id: VideoId::new(video_id)?,
            title,
            status: VideoStatus::parse(&status)?,
            quality: quality.map(Quality::new).transpose()?,
            filename,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .context("failed to parse stored created_at")?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_at)
                .context("failed to parse stored updated_at")?
                .with_timezone(&Utc),
        })
    }
}

impl VideoRepository for SqliteVideoRepository {
    fn save(&self, video: &Video) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "INSERT INTO videos (playlist_id, video_id, title, status, quality, filename, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT (playlist_id, video_id) DO UPDATE SET
                title = excluded.title,
                status = excluded.status,
                quality = excluded.quality,
                filename = excluded.filename,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at",
            params![
                video.playlist_id.as_str(),
                video.video_id.as_str(),
                video.title,
                video.status.as_str(),
                video.quality.map(|q| q.as_str()),
                video.filename,
                video.created_at.to_rfc3339(),
                video.updated_at.to_rfc3339(),
            ],
        )
        .context("failed to save video")?;
        Ok(())
    }

    fn find(&self, playlist_id: &PlaylistId, video_id: &VideoId) -> anyhow::Result<Option<Video>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.query_row(
            "SELECT playlist_id, video_id, title, status, quality, filename, created_at, updated_at
             FROM videos WHERE playlist_id = ?1 AND video_id = ?2",
            params![playlist_id.as_str(), video_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .optional()
        .context("failed to find video")?
        .map(
            |(playlist_id, video_id, title, status, quality, filename, created_at, updated_at)| {
                Self::row_to_video(
                    playlist_id,
                    video_id,
                    title,
                    status,
                    quality,
                    filename,
                    created_at,
                    updated_at,
                )
            },
        )
        .transpose()
    }

    fn list_for_playlist(&self, playlist_id: &PlaylistId) -> anyhow::Result<Vec<Video>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut stmt = conn
            .prepare(
                "SELECT playlist_id, video_id, title, status, quality, filename, created_at, updated_at
                 FROM videos WHERE playlist_id = ?1 ORDER BY video_id ASC",
            )
            .context("failed to prepare list-videos query")?;
        let rows = stmt
            .query_map(params![playlist_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })
            .context("failed to list videos")?;

        rows.map(|row| {
            let (playlist_id, video_id, title, status, quality, filename, created_at, updated_at) =
                row.context("failed to read video row")?;
            Self::row_to_video(
                playlist_id,
                video_id,
                title,
                status,
                quality,
                filename,
                created_at,
                updated_at,
            )
        })
        .collect()
    }

    fn delete(&self, playlist_id: &PlaylistId, video_id: &VideoId) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "DELETE FROM videos WHERE playlist_id = ?1 AND video_id = ?2",
            params![playlist_id.as_str(), video_id.as_str()],
        )
        .context("failed to delete video")?;
        Ok(())
    }

    fn update(&self, video: &Video) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "UPDATE videos SET title = ?3, status = ?4, quality = ?5, filename = ?6, updated_at = ?7
             WHERE playlist_id = ?1 AND video_id = ?2",
            params![
                video.playlist_id.as_str(),
                video.video_id.as_str(),
                video.title,
                video.status.as_str(),
                video.quality.map(|q| q.as_str()),
                video.filename,
                video.updated_at.to_rfc3339(),
            ],
        )
        .context("failed to update video")?;
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeVideoRepository {
    pub(crate) videos: Mutex<Vec<Video>>,
}

#[cfg(test)]
impl VideoRepository for FakeVideoRepository {
    fn save(&self, video: &Video) -> anyhow::Result<()> {
        let mut videos = self.videos.lock().unwrap();
        if let Some(existing) = videos
            .iter_mut()
            .find(|v| v.playlist_id == video.playlist_id && v.video_id == video.video_id)
        {
            *existing = video.clone();
        } else {
            videos.push(video.clone());
        }
        Ok(())
    }

    fn find(&self, playlist_id: &PlaylistId, video_id: &VideoId) -> anyhow::Result<Option<Video>> {
        Ok(self
            .videos
            .lock()
            .unwrap()
            .iter()
            .find(|v| v.playlist_id == *playlist_id && v.video_id == *video_id)
            .cloned())
    }

    fn list_for_playlist(&self, playlist_id: &PlaylistId) -> anyhow::Result<Vec<Video>> {
        Ok(self
            .videos
            .lock()
            .unwrap()
            .iter()
            .filter(|v| v.playlist_id == *playlist_id)
            .cloned()
            .collect())
    }

    fn delete(&self, playlist_id: &PlaylistId, video_id: &VideoId) -> anyhow::Result<()> {
        self.videos
            .lock()
            .unwrap()
            .retain(|v| v.playlist_id != *playlist_id || v.video_id != *video_id);
        Ok(())
    }

    fn update(&self, video: &Video) -> anyhow::Result<()> {
        let mut videos = self.videos.lock().unwrap();
        if let Some(existing) = videos
            .iter_mut()
            .find(|v| v.playlist_id == video.playlist_id && v.video_id == video.video_id)
        {
            *existing = video.clone();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> SqliteVideoRepository {
        SqliteVideoRepository::new(Connection::open_in_memory().unwrap()).unwrap()
    }

    #[test]
    fn it_should_be_idempotent_when_constructed_twice_against_an_already_migrated_database() {
        let db_path = std::env::temp_dir().join(format!(
            "yarrtube-video-repo-migration-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        SqliteVideoRepository::new(Connection::open(&db_path).unwrap()).unwrap();
        let result = SqliteVideoRepository::new(Connection::open(&db_path).unwrap());

        assert!(result.is_ok());
        std::fs::remove_file(&db_path).unwrap();
    }

    fn playlist_id() -> PlaylistId {
        PlaylistId::new("PL1").unwrap()
    }

    fn video(video_id: &str, title: &str, now: DateTime<Utc>) -> Video {
        Video::create(playlist_id(), VideoId::new(video_id).unwrap(), title, now)
    }

    #[test]
    fn it_should_return_the_video_after_saving_it() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        repo.save(&video("vid1", "First", now)).unwrap();

        let videos = repo.list_for_playlist(&playlist_id()).unwrap();
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].title, "First");
        assert_eq!(videos[0].status, VideoStatus::Pending);
        assert_eq!(videos[0].quality, None);
    }

    #[test]
    fn it_should_round_trip_a_video_with_no_recorded_quality() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        repo.save(&video("vid1", "First", now)).unwrap();

        let found = repo
            .find(&playlist_id(), &VideoId::new("vid1").unwrap())
            .unwrap()
            .unwrap();

        assert_eq!(found.quality, None);
    }

    #[test]
    fn it_should_round_trip_a_video_with_a_recorded_quality() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let downloaded = video("vid1", "First", now)
            .start_download(now)
            .mark_downloaded(Quality::Mid, "First.mp4", now);
        repo.save(&downloaded).unwrap();

        let found = repo
            .find(&playlist_id(), &VideoId::new("vid1").unwrap())
            .unwrap()
            .unwrap();

        assert_eq!(found.quality, Some(Quality::Mid));
        assert_eq!(found.filename, Some("First.mp4".to_string()));
    }

    #[test]
    fn it_should_overwrite_every_field_including_status_when_saving_an_existing_video() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        repo.save(&video("vid1", "First", now)).unwrap();

        let later = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let updated = video("vid1", "Renamed", later).start_download(later);
        repo.save(&updated).unwrap();

        let videos = repo.list_for_playlist(&playlist_id()).unwrap();
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].title, "Renamed");
        assert_eq!(videos[0].status, VideoStatus::InProgress);
        assert_eq!(videos[0].updated_at, later);
    }

    #[test]
    fn it_should_delete_only_the_named_video() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        repo.save(&video("vid1", "One", now)).unwrap();
        repo.save(&video("vid2", "Two", now)).unwrap();

        repo.delete(&playlist_id(), &VideoId::new("vid1").unwrap())
            .unwrap();

        let videos = repo.list_for_playlist(&playlist_id()).unwrap();
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].video_id.as_str(), "vid2");
    }

    #[test]
    fn it_should_succeed_when_deleting_a_missing_video() {
        let repo = repo();

        assert!(
            repo.delete(&playlist_id(), &VideoId::new("vid1").unwrap())
                .is_ok()
        );
    }

    #[test]
    fn it_should_return_none_when_finding_a_missing_video() {
        let repo = repo();

        let found = repo
            .find(&playlist_id(), &VideoId::new("vid1").unwrap())
            .unwrap();

        assert!(found.is_none());
    }

    #[test]
    fn it_should_return_the_stored_video_with_its_updated_status_after_update() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        repo.save(&video("vid1", "First", now)).unwrap();
        let stored = repo
            .find(&playlist_id(), &VideoId::new("vid1").unwrap())
            .unwrap()
            .unwrap();

        let later = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        repo.update(&stored.start_download(later)).unwrap();

        let found = repo
            .find(&playlist_id(), &VideoId::new("vid1").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(found.status, VideoStatus::InProgress);
        assert_eq!(found.updated_at, later);
    }
}
