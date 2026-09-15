use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
use crate::domain::shared::Quality;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::Mutex;

pub trait ChannelRepository: Send + Sync {
    fn find(&self, id: &ChannelHandle) -> anyhow::Result<Option<Channel>>;
    fn insert(&self, channel: &Channel) -> anyhow::Result<()>;
    fn delete(&self, id: &ChannelHandle) -> anyhow::Result<()>;
    fn list(&self) -> anyhow::Result<Vec<Channel>>;
}

pub struct SqliteChannelRepository {
    conn: Mutex<Connection>,
}

impl SqliteChannelRepository {
    pub fn new(conn: Connection) -> anyhow::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS channels (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                youtube_channel_id TEXT NOT NULL,
                quality TEXT NOT NULL,
                video_limit INTEGER NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )
        .inspect_err(|e| tracing::error!(error = %e, "failed to create channels table"))
        .context("failed to create channels table")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn row_to_channel(
        id: String,
        name: String,
        youtube_channel_id: String,
        quality: String,
        video_limit: i64,
        created_at: String,
    ) -> anyhow::Result<Channel> {
        let id = ChannelHandle::new(id)?;
        let quality = Quality::new(quality)?;
        let video_limit = VideoLimit::new(video_limit)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .context("failed to parse stored created_at")?
            .with_timezone(&Utc);
        Ok(Channel::create(
            id,
            name,
            youtube_channel_id,
            quality,
            video_limit,
            created_at,
        ))
    }
}

impl ChannelRepository for SqliteChannelRepository {
    fn find(&self, id: &ChannelHandle) -> anyhow::Result<Option<Channel>> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(channel_id = %id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.query_row(
            "SELECT id, name, youtube_channel_id, quality, video_limit, created_at FROM channels WHERE id = ?1",
            params![id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .inspect_err(|e| tracing::error!(channel_id = %id, error = %e, "failed to query channel"))
        .context("failed to query channel")?
        .map(|(id, name, youtube_channel_id, quality, video_limit, created_at)| {
            Self::row_to_channel(id, name, youtube_channel_id, quality, video_limit, created_at)
        })
        .transpose()
    }

    fn insert(&self, channel: &Channel) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(channel_id = %channel.id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "INSERT INTO channels (id, name, youtube_channel_id, quality, video_limit, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                channel.id.as_str(),
                channel.name,
                channel.youtube_channel_id,
                channel.quality.as_str(),
                channel.video_limit.value(),
                channel.created_at.to_rfc3339()
            ],
        )
        .inspect_err(|e| tracing::error!(channel_id = %channel.id, error = %e, "failed to insert channel"))
        .context("failed to insert channel")?;
        Ok(())
    }

    fn delete(&self, id: &ChannelHandle) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(channel_id = %id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute("DELETE FROM channels WHERE id = ?1", params![id.as_str()])
            .inspect_err(
                |e| tracing::error!(channel_id = %id, error = %e, "failed to delete channel"),
            )
            .context("failed to delete channel")?;
        Ok(())
    }

    fn list(&self) -> anyhow::Result<Vec<Channel>> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!("database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, youtube_channel_id, quality, video_limit, created_at FROM channels ORDER BY rowid ASC",
            )
            .inspect_err(|e| tracing::error!(error = %e, "failed to prepare list query"))
            .context("failed to prepare list query")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .inspect_err(|e| tracing::error!(error = %e, "failed to list channels"))
            .context("failed to list channels")?;

        rows.map(|row| {
            let (id, name, youtube_channel_id, quality, video_limit, created_at) = row
                .inspect_err(|e| tracing::error!(error = %e, "failed to read channel row"))
                .context("failed to read channel row")?;
            Self::row_to_channel(
                id,
                name,
                youtube_channel_id,
                quality,
                video_limit,
                created_at,
            )
        })
        .collect()
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeChannelRepository {
    channels: Mutex<Vec<Channel>>,
}

#[cfg(test)]
impl ChannelRepository for FakeChannelRepository {
    fn find(&self, id: &ChannelHandle) -> anyhow::Result<Option<Channel>> {
        Ok(self
            .channels
            .lock()
            .unwrap()
            .iter()
            .find(|c| c.id == *id)
            .cloned())
    }

    fn insert(&self, channel: &Channel) -> anyhow::Result<()> {
        self.channels.lock().unwrap().push(channel.clone());
        Ok(())
    }

    fn delete(&self, id: &ChannelHandle) -> anyhow::Result<()> {
        self.channels.lock().unwrap().retain(|c| c.id != *id);
        Ok(())
    }

    fn list(&self) -> anyhow::Result<Vec<Channel>> {
        Ok(self.channels.lock().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> SqliteChannelRepository {
        SqliteChannelRepository::new(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn channel(id: &str, name: &str) -> Channel {
        Channel::create(
            ChannelHandle::new(id).unwrap(),
            name,
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
    }

    #[test]
    fn it_should_return_none_when_the_channel_does_not_exist() {
        let repo = repo();
        let id = ChannelHandle::new("@missing").unwrap();

        assert_eq!(repo.find(&id).unwrap(), None);
    }

    #[test]
    fn it_should_return_the_channel_after_inserting_it() {
        let repo = repo();
        let channel = channel("@somechannel", "Some Channel");

        repo.insert(&channel).unwrap();

        assert_eq!(repo.find(&channel.id).unwrap(), Some(channel));
    }

    #[test]
    fn it_should_fail_when_inserting_a_duplicate_id() {
        let repo = repo();
        let channel = channel("@somechannel", "Some Channel");
        repo.insert(&channel).unwrap();

        assert!(repo.insert(&channel).is_err());
    }

    #[test]
    fn it_should_remove_an_existing_channel() {
        let repo = repo();
        let channel = channel("@somechannel", "Some Channel");
        repo.insert(&channel).unwrap();

        repo.delete(&channel.id).unwrap();

        assert_eq!(repo.find(&channel.id).unwrap(), None);
    }

    #[test]
    fn it_should_succeed_when_deleting_a_missing_channel() {
        let repo = repo();
        let id = ChannelHandle::new("@missing").unwrap();

        assert!(repo.delete(&id).is_ok());
    }

    #[test]
    fn it_should_return_an_empty_list_when_no_channels_are_saved() {
        let repo = repo();

        assert_eq!(repo.list().unwrap(), Vec::new());
    }

    #[test]
    fn it_should_return_all_saved_channels() {
        let repo = repo();
        let first = channel("@first", "First");
        let second = channel("@second", "Second");
        repo.insert(&first).unwrap();
        repo.insert(&second).unwrap();

        assert_eq!(repo.list().unwrap(), vec![first, second]);
    }
}
