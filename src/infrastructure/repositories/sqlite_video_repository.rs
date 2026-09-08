use crate::domain::playlist::YoutubePlaylistId;
use crate::domain::video::{Video, VideoStatus, YoutubeVideoId};
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params, params_from_iter};
use std::sync::Mutex;

pub trait VideoRepository: Send + Sync {
    fn upsert(&self, video: &Video) -> anyhow::Result<()>;
    fn list_for_playlist(&self, playlist_id: &YoutubePlaylistId) -> anyhow::Result<Vec<Video>>;
    /// Deletes every stored video for `playlist_id` whose YouTube ID is not
    /// in `current_ids`.
    fn delete_not_in(
        &self,
        playlist_id: &YoutubePlaylistId,
        current_ids: &[YoutubeVideoId],
    ) -> anyhow::Result<()>;
}

pub struct SqliteVideoRepository {
    conn: Mutex<Connection>,
}

impl SqliteVideoRepository {
    pub fn new(conn: Connection) -> anyhow::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS videos (
                playlist_id TEXT NOT NULL,
                youtube_video_id TEXT NOT NULL,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (playlist_id, youtube_video_id)
            )",
            [],
        )
        .context("failed to create videos table")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn row_to_video(
        playlist_id: String,
        youtube_video_id: String,
        title: String,
        status: String,
        created_at: String,
        updated_at: String,
    ) -> anyhow::Result<Video> {
        Ok(Video {
            playlist_id: YoutubePlaylistId::new(playlist_id)?,
            youtube_video_id: YoutubeVideoId::new(youtube_video_id)?,
            title,
            status: VideoStatus::parse(&status)?,
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
    fn upsert(&self, video: &Video) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "INSERT INTO videos (playlist_id, youtube_video_id, title, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT (playlist_id, youtube_video_id) DO UPDATE SET
                title = excluded.title,
                updated_at = excluded.updated_at",
            params![
                video.playlist_id.as_str(),
                video.youtube_video_id.as_str(),
                video.title,
                video.status.as_str(),
                video.updated_at.to_rfc3339(),
            ],
        )
        .context("failed to upsert video")?;
        Ok(())
    }

    fn list_for_playlist(&self, playlist_id: &YoutubePlaylistId) -> anyhow::Result<Vec<Video>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut stmt = conn
            .prepare(
                "SELECT playlist_id, youtube_video_id, title, status, created_at, updated_at
                 FROM videos WHERE playlist_id = ?1 ORDER BY youtube_video_id ASC",
            )
            .context("failed to prepare list-videos query")?;
        let rows = stmt
            .query_map(params![playlist_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .context("failed to list videos")?;

        rows.map(|row| {
            let (playlist_id, youtube_video_id, title, status, created_at, updated_at) =
                row.context("failed to read video row")?;
            Self::row_to_video(
                playlist_id,
                youtube_video_id,
                title,
                status,
                created_at,
                updated_at,
            )
        })
        .collect()
    }

    fn delete_not_in(
        &self,
        playlist_id: &YoutubePlaylistId,
        current_ids: &[YoutubeVideoId],
    ) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;

        if current_ids.is_empty() {
            conn.execute(
                "DELETE FROM videos WHERE playlist_id = ?1",
                params![playlist_id.as_str()],
            )
            .context("failed to delete videos for playlist")?;
            return Ok(());
        }

        let placeholders = (2..=current_ids.len() + 1)
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "DELETE FROM videos WHERE playlist_id = ?1 AND youtube_video_id NOT IN ({placeholders})"
        );
        let mut all_params: Vec<String> = vec![playlist_id.as_str().to_string()];
        all_params.extend(current_ids.iter().map(|id| id.as_str().to_string()));

        conn.execute(&sql, params_from_iter(all_params.iter()))
            .context("failed to delete removed videos")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> SqliteVideoRepository {
        SqliteVideoRepository::new(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn playlist_id() -> YoutubePlaylistId {
        YoutubePlaylistId::new("PL1").unwrap()
    }

    fn video(video_id: &str, title: &str, now: DateTime<Utc>) -> Video {
        Video::create(
            playlist_id(),
            YoutubeVideoId::new(video_id).unwrap(),
            title,
            now,
        )
    }

    #[test]
    fn it_should_return_the_video_after_upserting_it() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        repo.upsert(&video("vid1", "First", now)).unwrap();

        let videos = repo.list_for_playlist(&playlist_id()).unwrap();
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].title, "First");
        assert_eq!(videos[0].status, VideoStatus::Pending);
    }

    #[test]
    fn it_should_leave_the_status_untouched_when_upserting_an_existing_video() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        repo.upsert(&video("vid1", "First", now)).unwrap();

        let later = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        repo.upsert(&video("vid1", "Renamed", later)).unwrap();

        let videos = repo.list_for_playlist(&playlist_id()).unwrap();
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].title, "Renamed");
        assert_eq!(videos[0].status, VideoStatus::Pending);
        assert_eq!(videos[0].updated_at, later);
    }

    #[test]
    fn it_should_delete_videos_no_longer_in_the_current_set() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        repo.upsert(&video("vid1", "One", now)).unwrap();
        repo.upsert(&video("vid2", "Two", now)).unwrap();

        repo.delete_not_in(&playlist_id(), &[YoutubeVideoId::new("vid1").unwrap()])
            .unwrap();

        let videos = repo.list_for_playlist(&playlist_id()).unwrap();
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].youtube_video_id.as_str(), "vid1");
    }

    #[test]
    fn it_should_delete_all_videos_when_the_current_set_is_empty() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        repo.upsert(&video("vid1", "One", now)).unwrap();

        repo.delete_not_in(&playlist_id(), &[]).unwrap();

        assert!(repo.list_for_playlist(&playlist_id()).unwrap().is_empty());
    }
}
