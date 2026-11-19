use crate::domain::show::ShowKey;
use crate::domain::video::{DiscardedVideo, YoutubeVideoId};
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::params;

/// The Discarded videos a reconcile removed from their source, remembered so
/// the source never adds them back.
pub trait DiscardedVideoRepository: Send + Sync {
    /// Idempotent.
    fn record(&self, discarded: &DiscardedVideo) -> anyhow::Result<()>;
    fn list(&self, source: &ShowKey) -> anyhow::Result<Vec<DiscardedVideo>>;
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
    fn record(&self, discarded: &DiscardedVideo) -> anyhow::Result<()> {
        let (source_kind, source_id) = Self::source_columns(&discarded.source);
        let youtube_id = &discarded.youtube_id;
        let conn = self.db.write()?;
        conn.execute(
            "INSERT INTO discarded_videos (source_kind, source_id, youtube_id, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (source_kind, source_id, youtube_id) DO NOTHING",
            params![
                source_kind,
                source_id,
                youtube_id.as_str(),
                discarded.created_at.to_rfc3339()
            ],
        )
        .inspect_err(|e| {
            tracing::error!(source_kind, source_id, youtube_id = %youtube_id, error = %e, "failed to record discarded video")
        })
        .context("failed to record discarded video")?;
        Ok(())
    }

    fn list(&self, source: &ShowKey) -> anyhow::Result<Vec<DiscardedVideo>> {
        let (source_kind, source_id) = Self::source_columns(source);
        let conn = self.db.read()?;
        let rows = conn
            .prepare(
                "SELECT youtube_id, created_at FROM discarded_videos
                 WHERE source_kind = ?1 AND source_id = ?2 ORDER BY youtube_id",
            )
            .and_then(|mut stmt| {
                stmt.query_map(params![source_kind, source_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()
            })
            .inspect_err(|e| {
                tracing::error!(source_kind, source_id, error = %e, "failed to list discarded videos")
            })
            .context("failed to list discarded videos")?;
        rows.into_iter()
            .map(|(youtube_id, created_at)| {
                Ok(DiscardedVideo::new(
                    source.clone(),
                    YoutubeVideoId::new(youtube_id)?,
                    DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
                ))
            })
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
        let recorded = [
            discarded(&channel, "yt1"),
            discarded(&channel, "yt2"),
            discarded(&playlist, "yt3"),
        ];
        recorded
            .iter()
            .for_each(|discarded| repository.record(discarded).unwrap());

        let listed = [&channel, &playlist].map(|source| repository.list(source).unwrap());
        repository.delete_all(&channel).unwrap();
        let after_delete = [&channel, &playlist].map(|source| repository.list(source).unwrap());

        let [yt1, yt2, yt3] = recorded;
        assert_eq!(
            (listed, after_delete),
            ([vec![yt1, yt2], vec![yt3.clone()]], [vec![], vec![yt3]])
        );
    }

    #[test]
    fn it_should_record_an_id_once() {
        let db = TestDatabase::new();
        let repository = SqliteDiscardedVideoRepository::new(db.database());
        let source = channel("@somechannel");
        repository.record(&discarded(&source, "yt1")).unwrap();

        let result = repository.record(&discarded(&source, "yt1"));

        assert_eq!(
            (
                result.map_err(|e| e.to_string()),
                repository.list(&source).unwrap()
            ),
            (Ok(()), vec![discarded(&source, "yt1")])
        );
    }

    fn channel(handle: &str) -> ShowKey {
        ShowKey::Channel(ChannelHandle::new(handle).unwrap())
    }

    fn playlist(id: &str) -> ShowKey {
        ShowKey::Playlist(PlaylistId::new(id).unwrap())
    }

    fn discarded(source: &ShowKey, youtube_id: &str) -> DiscardedVideo {
        DiscardedVideo::new(
            source.clone(),
            YoutubeVideoId::new(youtube_id).unwrap(),
            now(),
        )
    }

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }
}
