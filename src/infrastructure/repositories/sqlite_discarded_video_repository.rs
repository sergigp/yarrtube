use crate::domain::show::ShowKey;
use crate::domain::video::YoutubeVideoId;
use crate::infrastructure::shared::sqlite_connection::Database;
use chrono::{DateTime, Utc};
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
    #[allow(dead_code)]
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
        _source: &ShowKey,
        _youtube_id: &YoutubeVideoId,
        _now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn list(&self, _source: &ShowKey) -> anyhow::Result<HashSet<YoutubeVideoId>> {
        Ok(HashSet::new())
    }

    fn delete_all(&self, _source: &ShowKey) -> anyhow::Result<()> {
        Ok(())
    }
}
