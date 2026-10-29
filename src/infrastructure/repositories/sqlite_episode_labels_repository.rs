use crate::domain::plex_label::EpisodeLabels;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::shared::sqlite_connection::Database;

/// The Plex label state of each stored video record.
pub trait EpisodeLabelsRepository: Send + Sync {
    /// Only the ids that have a row.
    fn find_many(&self, ids: &[VideoRecordId]) -> anyhow::Result<Vec<EpisodeLabels>>;
    /// Dirty rows, never checked first then least recently checked, at most
    /// `limit`.
    fn list_dirty(&self, limit: usize) -> anyhow::Result<Vec<EpisodeLabels>>;
    /// Rows with a rating key.
    fn list_synced(&self) -> anyhow::Result<Vec<EpisodeLabels>>;
    /// Inserts or replaces each row, all in one transaction.
    fn save_many(&self, rows: &[EpisodeLabels]) -> anyhow::Result<()>;
    fn delete(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
}

pub struct SqliteEpisodeLabelsRepository {
    db: Database,
}

impl SqliteEpisodeLabelsRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl EpisodeLabelsRepository for SqliteEpisodeLabelsRepository {
    fn find_many(&self, _ids: &[VideoRecordId]) -> anyhow::Result<Vec<EpisodeLabels>> {
        Ok(Vec::new())
    }

    fn list_dirty(&self, _limit: usize) -> anyhow::Result<Vec<EpisodeLabels>> {
        Ok(Vec::new())
    }

    fn list_synced(&self) -> anyhow::Result<Vec<EpisodeLabels>> {
        Ok(Vec::new())
    }

    fn save_many(&self, _rows: &[EpisodeLabels]) -> anyhow::Result<()> {
        Ok(())
    }

    fn delete(&self, _video_id: &VideoRecordId) -> anyhow::Result<()> {
        Ok(())
    }
}
