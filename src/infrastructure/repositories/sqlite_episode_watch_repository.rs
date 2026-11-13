use crate::domain::plex_watch::EpisodeWatch;
use crate::domain::video::VideoId;
use crate::infrastructure::shared::sqlite_connection::Database;

/// The watch state each stored video last agreed on with its Plex episode.
pub trait EpisodeWatchRepository: Send + Sync {
    fn list(&self) -> anyhow::Result<Vec<EpisodeWatch>>;
    /// Inserts or replaces each row by video id, all in one transaction.
    fn save_many(&self, rows: &[EpisodeWatch]) -> anyhow::Result<()>;
    fn delete_many(&self, video_ids: &[VideoId]) -> anyhow::Result<()>;
}

pub struct SqliteEpisodeWatchRepository {
    #[allow(dead_code)]
    db: Database,
}

impl SqliteEpisodeWatchRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl EpisodeWatchRepository for SqliteEpisodeWatchRepository {
    fn list(&self) -> anyhow::Result<Vec<EpisodeWatch>> {
        Ok(vec![])
    }

    fn save_many(&self, _rows: &[EpisodeWatch]) -> anyhow::Result<()> {
        Ok(())
    }

    fn delete_many(&self, _video_ids: &[VideoId]) -> anyhow::Result<()> {
        Ok(())
    }
}
