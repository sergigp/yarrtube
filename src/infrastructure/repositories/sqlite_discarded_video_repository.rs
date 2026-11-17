use crate::domain::show::ShowKey;
use crate::domain::video::YoutubeVideoId;
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::params;
use std::collections::HashSet;

/// The Discarded videos a reconcile removed from their source, remembered so
/// the source never adds them back.
pub trait DiscardedVideoRepository: Send + Sync {
    /// Idempotent.
    fn record(
        &self,
        source: &ShowKey,
        youtube_id: &YoutubeVideoId,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()>;
    fn list(&self, source: &ShowKey) -> anyhow::Result<HashSet<YoutubeVideoId>>;
    fn delete_all(&self, source: &ShowKey) -> anyhow::Result<()>;
}

pub struct SqliteDiscardedVideoRepository {
    db: Database,
}

impl SqliteDiscardedVideoRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl DiscardedVideoRepository for SqliteDiscardedVideoRepository {
    fn record(
        &self,
        source: &ShowKey,
        youtube_id: &YoutubeVideoId,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let (source_kind, source_id) = Self::source_columns(source);
        let conn = self.db.write()?;
        conn.execute(
            "INSERT INTO discarded_videos (source_kind, source_id, youtube_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![source_kind, source_id, youtube_id.as_str(), now.to_rfc3339()],
        )
        .inspect_err(|e| {
            tracing::error!(source_kind, source_id, youtube_id = %youtube_id, error = %e, "failed to record discarded video")
        })
        .context("failed to record discarded video")?;
        Ok(())
    }

    fn list(&self, source: &ShowKey) -> anyhow::Result<HashSet<YoutubeVideoId>> {
        let (source_kind, source_id) = Self::source_columns(source);
        let conn = self.db.read()?;
        let youtube_ids = conn
            .prepare(
                "SELECT youtube_id FROM discarded_videos WHERE source_kind = ?1 AND source_id = ?2",
            )
            .and_then(|mut stmt| {
                stmt.query_map(params![source_kind, source_id], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<Result<Vec<_>, _>>()
            })
            .inspect_err(|e| {
                tracing::error!(source_kind, source_id, error = %e, "failed to list discarded videos")
            })
            .context("failed to list discarded videos")?;
        youtube_ids
            .into_iter()
            .map(|id| YoutubeVideoId::new(id).map_err(anyhow::Error::from))
            .collect()
    }

    fn delete_all(&self, source: &ShowKey) -> anyhow::Result<()> {
        let (source_kind, source_id) = Self::source_columns(source);
        let conn = self.db.write()?;
        conn.execute(
            "DELETE FROM discarded_videos WHERE source_kind = ?1 AND source_id = ?2",
            params![source_kind, source_id],
        )
        .inspect_err(|e| {
            tracing::error!(source_kind, source_id, error = %e, "failed to delete discarded videos")
        })
        .context("failed to delete discarded videos")?;
        Ok(())
    }
}

impl SqliteDiscardedVideoRepository {
    /// The `source_kind` and `source_id` columns identifying a source.
    fn source_columns(source: &ShowKey) -> (&'static str, &str) {
        match source {
            ShowKey::Channel(handle) => ("channel", handle.as_str()),
            ShowKey::Playlist(id) => ("playlist", id.as_str()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::ChannelHandle;
    use crate::domain::playlist::PlaylistId;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;

    #[test]
    fn it_should_list_the_recorded_ids_per_source() {
        let db = TestDatabase::new();
        let repository = SqliteDiscardedVideoRepository::new(db.database());
        let (channel, playlist) = (channel("@somechannel"), playlist("PL1"));
        repository
            .record(&channel, &youtube_id("yt1"), now())
            .unwrap();
        repository
            .record(&channel, &youtube_id("yt2"), now())
            .unwrap();
        repository
            .record(&playlist, &youtube_id("yt3"), now())
            .unwrap();

        let listed = [&channel, &playlist].map(|source| repository.list(source).unwrap());
        repository.delete_all(&channel).unwrap();
        let after_delete = [&channel, &playlist].map(|source| repository.list(source).unwrap());

        assert_eq!(
            (listed, after_delete),
            (
                [ids(&["yt1", "yt2"]), ids(&["yt3"])],
                [ids(&[]), ids(&["yt3"])]
            )
        );
    }

    fn channel(handle: &str) -> ShowKey {
        ShowKey::Channel(ChannelHandle::new(handle).unwrap())
    }

    fn playlist(id: &str) -> ShowKey {
        ShowKey::Playlist(PlaylistId::new(id).unwrap())
    }

    fn youtube_id(id: &str) -> YoutubeVideoId {
        YoutubeVideoId::new(id).unwrap()
    }

    fn ids(ids: &[&str]) -> HashSet<YoutubeVideoId> {
        ids.iter().map(|id| youtube_id(id)).collect()
    }

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }
}
