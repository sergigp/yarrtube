use crate::domain::channel::ChannelHandle;
use crate::domain::channel_video::ChannelVideo;
use crate::domain::shared::{VideoId, VideoRecordId};
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::Mutex;

pub trait ChannelVideoRepository: Send + Sync {
    /// Insert-or-replace keyed by `(channel_id, video_id)`.
    fn save(&self, channel_video: &ChannelVideo) -> anyhow::Result<()>;
    fn find_by_youtube_video(
        &self,
        channel_id: &ChannelHandle,
        youtube_video_id: &VideoId,
    ) -> anyhow::Result<Option<ChannelVideo>>;
    /// Finds whichever channel a video belongs to, keyed by the video's own
    /// surrogate ID rather than a `(channel_id, youtube_video_id)` pair.
    fn find_by_video(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<ChannelVideo>>;
    /// Ordered by recency position (`0` = most recent).
    fn list_for_channel(&self, channel_id: &ChannelHandle) -> anyhow::Result<Vec<ChannelVideo>>;
    fn delete(&self, channel_id: &ChannelHandle, youtube_video_id: &VideoId) -> anyhow::Result<()>;
    fn delete_all_for_channel(&self, channel_id: &ChannelHandle) -> anyhow::Result<()>;
}

pub struct SqliteChannelVideoRepository {
    conn: Mutex<Connection>,
}

impl SqliteChannelVideoRepository {
    pub fn new(conn: Connection) -> anyhow::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS channel_videos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_id TEXT NOT NULL,
                video_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE (channel_id, video_id)
            )",
            [],
        )
        .inspect_err(|e| tracing::error!(error = %e, "failed to create channel_videos table"))
        .context("failed to create channel_videos table")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

impl ChannelVideoRepository for SqliteChannelVideoRepository {
    fn save(&self, channel_video: &ChannelVideo) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| {
                tracing::error!(channel_id = %channel_video.channel_id, "database lock poisoned")
            })
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "INSERT INTO channel_videos (channel_id, video_id, position, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (channel_id, video_id) DO UPDATE SET
                position = excluded.position,
                created_at = excluded.created_at",
            params![
                channel_video.channel_id.as_str(),
                channel_video.video_id.as_str(),
                channel_video.position,
                channel_video.created_at.to_rfc3339(),
            ],
        )
        .inspect_err(|e| {
            tracing::error!(channel_id = %channel_video.channel_id, error = %e, "failed to save channel video")
        })
        .context("failed to save channel video")?;
        Ok(())
    }

    fn find_by_youtube_video(
        &self,
        channel_id: &ChannelHandle,
        youtube_video_id: &VideoId,
    ) -> anyhow::Result<Option<ChannelVideo>> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(channel_id = %channel_id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.query_row(
            "SELECT cv.id, cv.channel_id, cv.video_id, cv.position, cv.created_at
             FROM channel_videos cv
             JOIN videos v ON v.id = cv.video_id
             WHERE cv.channel_id = ?1 AND v.youtube_id = ?2",
            params![channel_id.as_str(), youtube_video_id.as_str()],
            row_to_columns,
        )
        .optional()
        .inspect_err(|e| {
            tracing::error!(channel_id = %channel_id, error = %e, "failed to find channel video")
        })
        .context("failed to find channel video")?
        .map(columns_to_channel_video)
        .transpose()
    }

    fn find_by_video(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<ChannelVideo>> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(video_id = %video_id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.query_row(
            "SELECT id, channel_id, video_id, position, created_at
             FROM channel_videos WHERE video_id = ?1",
            params![video_id.as_str()],
            row_to_columns,
        )
        .optional()
        .inspect_err(|e| {
            tracing::error!(video_id = %video_id, error = %e, "failed to find channel video by video")
        })
        .context("failed to find channel video by video")?
        .map(columns_to_channel_video)
        .transpose()
    }

    fn list_for_channel(&self, channel_id: &ChannelHandle) -> anyhow::Result<Vec<ChannelVideo>> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(channel_id = %channel_id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, channel_id, video_id, position, created_at
                 FROM channel_videos WHERE channel_id = ?1
                 ORDER BY position ASC",
            )
            .inspect_err(|e| {
                tracing::error!(channel_id = %channel_id, error = %e, "failed to prepare list-channel-videos query")
            })
            .context("failed to prepare list-channel-videos query")?;
        let rows = stmt
            .query_map(params![channel_id.as_str()], row_to_columns)
            .inspect_err(|e| {
                tracing::error!(channel_id = %channel_id, error = %e, "failed to list channel videos")
            })
            .context("failed to list channel videos")?;

        rows.map(|row| {
            let columns = row
                .inspect_err(|e| tracing::error!(error = %e, "failed to read channel video row"))
                .context("failed to read channel video row")?;
            columns_to_channel_video(columns)
        })
        .collect()
    }

    fn delete(&self, channel_id: &ChannelHandle, youtube_video_id: &VideoId) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(channel_id = %channel_id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "DELETE FROM channel_videos
             WHERE channel_id = ?1 AND video_id IN (
                SELECT id FROM videos WHERE youtube_id = ?2
             )",
            params![channel_id.as_str(), youtube_video_id.as_str()],
        )
        .inspect_err(|e| {
            tracing::error!(channel_id = %channel_id, error = %e, "failed to delete channel video")
        })
        .context("failed to delete channel video")?;
        Ok(())
    }

    fn delete_all_for_channel(&self, channel_id: &ChannelHandle) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(channel_id = %channel_id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "DELETE FROM channel_videos WHERE channel_id = ?1",
            params![channel_id.as_str()],
        )
        .inspect_err(|e| {
            tracing::error!(channel_id = %channel_id, error = %e, "failed to delete channel videos for channel")
        })
        .context("failed to delete channel videos for channel")?;
        Ok(())
    }
}

type Columns = (i64, String, String, i64, String);

fn row_to_columns(row: &rusqlite::Row) -> rusqlite::Result<Columns> {
    Ok((
        row.get::<_, i64>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, i64>(3)?,
        row.get::<_, String>(4)?,
    ))
}

fn columns_to_channel_video(columns: Columns) -> anyhow::Result<ChannelVideo> {
    let (id, channel_id, video_id, position, created_at) = columns;
    Ok(ChannelVideo {
        id,
        channel_id: ChannelHandle::new(channel_id)?,
        video_id: VideoRecordId::new(video_id)?,
        position,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .context("failed to parse stored created_at")?
            .with_timezone(&Utc),
    })
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeChannelVideoRepository {
    pub(crate) channel_videos: Mutex<Vec<ChannelVideo>>,
    #[allow(clippy::type_complexity)]
    pub(crate) youtube_ids: Mutex<std::collections::HashMap<String, VideoId>>,
}

#[cfg(test)]
impl FakeChannelVideoRepository {
    /// Test-only bookkeeping mirroring the SQL implementation's join with
    /// `videos`: lets the fake resolve `find_by_youtube_video` without a
    /// real `VideoRepository` alongside it.
    pub fn register_youtube_id(&self, video_id: &VideoRecordId, youtube_video_id: &VideoId) {
        self.youtube_ids
            .lock()
            .unwrap()
            .insert(video_id.as_str().to_string(), youtube_video_id.clone());
    }
}

#[cfg(test)]
impl ChannelVideoRepository for FakeChannelVideoRepository {
    fn save(&self, channel_video: &ChannelVideo) -> anyhow::Result<()> {
        let mut channel_videos = self.channel_videos.lock().unwrap();
        if let Some(existing) = channel_videos.iter_mut().find(|cv| {
            cv.channel_id == channel_video.channel_id && cv.video_id == channel_video.video_id
        }) {
            *existing = channel_video.clone();
        } else {
            channel_videos.push(channel_video.clone());
        }
        Ok(())
    }

    fn find_by_youtube_video(
        &self,
        channel_id: &ChannelHandle,
        youtube_video_id: &VideoId,
    ) -> anyhow::Result<Option<ChannelVideo>> {
        let youtube_ids = self.youtube_ids.lock().unwrap();
        Ok(self
            .channel_videos
            .lock()
            .unwrap()
            .iter()
            .find(|cv| {
                cv.channel_id == *channel_id
                    && youtube_ids.get(cv.video_id.as_str()) == Some(youtube_video_id)
            })
            .cloned())
    }

    fn find_by_video(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<ChannelVideo>> {
        Ok(self
            .channel_videos
            .lock()
            .unwrap()
            .iter()
            .find(|cv| cv.video_id == *video_id)
            .cloned())
    }

    fn list_for_channel(&self, channel_id: &ChannelHandle) -> anyhow::Result<Vec<ChannelVideo>> {
        let mut channel_videos: Vec<ChannelVideo> = self
            .channel_videos
            .lock()
            .unwrap()
            .iter()
            .filter(|cv| cv.channel_id == *channel_id)
            .cloned()
            .collect();
        channel_videos.sort_by_key(|cv| cv.position);
        Ok(channel_videos)
    }

    fn delete(&self, channel_id: &ChannelHandle, youtube_video_id: &VideoId) -> anyhow::Result<()> {
        let youtube_ids = self.youtube_ids.lock().unwrap();
        self.channel_videos.lock().unwrap().retain(|cv| {
            !(cv.channel_id == *channel_id
                && youtube_ids.get(cv.video_id.as_str()) == Some(youtube_video_id))
        });
        Ok(())
    }

    fn delete_all_for_channel(&self, channel_id: &ChannelHandle) -> anyhow::Result<()> {
        self.channel_videos
            .lock()
            .unwrap()
            .retain(|cv| cv.channel_id != *channel_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::video::Video;

    fn repo() -> SqliteChannelVideoRepository {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE videos (
                id TEXT PRIMARY KEY,
                youtube_id TEXT NOT NULL,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                quality TEXT,
                filename TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        SqliteChannelVideoRepository::new(conn).unwrap()
    }

    fn channel_id() -> ChannelHandle {
        ChannelHandle::new("@somechannel").unwrap()
    }

    fn seed_video(repo: &SqliteChannelVideoRepository, youtube_id: &str, title: &str) -> Video {
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
    fn it_should_return_the_channel_video_after_saving_it() {
        let repo = repo();
        let video = seed_video(&repo, "yt1", "First");
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        repo.save(&ChannelVideo::create(
            channel_id(),
            video.id.clone(),
            0,
            now,
        ))
        .unwrap();

        let found = repo
            .find_by_youtube_video(&channel_id(), &VideoId::new("yt1").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(found.video_id, video.id);
        assert_eq!(found.position, 0);
    }

    #[test]
    fn it_should_list_channel_videos_ordered_by_recency_position() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let newest = seed_video(&repo, "yt_newest", "Newest");
        let oldest = seed_video(&repo, "yt_oldest", "Oldest");
        repo.save(&ChannelVideo::create(
            channel_id(),
            oldest.id.clone(),
            1,
            now,
        ))
        .unwrap();
        repo.save(&ChannelVideo::create(
            channel_id(),
            newest.id.clone(),
            0,
            now,
        ))
        .unwrap();

        let videos = repo.list_for_channel(&channel_id()).unwrap();

        assert_eq!(
            videos
                .iter()
                .map(|cv| cv.video_id.clone())
                .collect::<Vec<_>>(),
            vec![newest.id, oldest.id]
        );
    }

    #[test]
    fn it_should_delete_only_the_named_channel_video() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let one = seed_video(&repo, "yt1", "One");
        let two = seed_video(&repo, "yt2", "Two");
        repo.save(&ChannelVideo::create(channel_id(), one.id.clone(), 0, now))
            .unwrap();
        repo.save(&ChannelVideo::create(channel_id(), two.id.clone(), 1, now))
            .unwrap();

        repo.delete(&channel_id(), &VideoId::new("yt1").unwrap())
            .unwrap();

        assert!(
            repo.find_by_youtube_video(&channel_id(), &VideoId::new("yt1").unwrap())
                .unwrap()
                .is_none()
        );
        assert!(
            repo.find_by_youtube_video(&channel_id(), &VideoId::new("yt2").unwrap())
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn it_should_delete_every_channel_video_for_the_channel_and_leave_others_untouched() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let one = seed_video(&repo, "yt1", "One");
        let two = seed_video(&repo, "yt2", "Two");
        repo.save(&ChannelVideo::create(channel_id(), one.id.clone(), 0, now))
            .unwrap();
        let other_channel_id = ChannelHandle::new("@other").unwrap();
        repo.save(&ChannelVideo::create(
            other_channel_id.clone(),
            two.id.clone(),
            0,
            now,
        ))
        .unwrap();

        repo.delete_all_for_channel(&channel_id()).unwrap();

        assert!(repo.list_for_channel(&channel_id()).unwrap().is_empty());
        assert_eq!(repo.list_for_channel(&other_channel_id).unwrap().len(), 1);
    }

    #[test]
    fn it_should_return_none_when_finding_a_missing_channel_video() {
        let repo = repo();

        let found = repo
            .find_by_youtube_video(&channel_id(), &VideoId::new("yt1").unwrap())
            .unwrap();

        assert!(found.is_none());
    }

    #[test]
    fn it_should_find_a_channel_video_by_its_video_record_id() {
        let repo = repo();
        let video = seed_video(&repo, "yt1", "First");
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        repo.save(&ChannelVideo::create(
            channel_id(),
            video.id.clone(),
            0,
            now,
        ))
        .unwrap();

        let found = repo.find_by_video(&video.id).unwrap().unwrap();

        assert_eq!(found.channel_id, channel_id());
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
