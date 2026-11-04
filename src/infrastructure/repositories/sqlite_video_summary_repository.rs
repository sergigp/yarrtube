use crate::domain::video::VideoRecordId;
use crate::domain::video_summary::VideoSummary;
use crate::infrastructure::shared::sqlite_connection::Database;

/// Stores each video's AI summary, at most one per video.
pub trait VideoSummaryRepository: Send + Sync {
    /// Inserts the summary, replacing the video's previous one.
    fn save(&self, summary: &VideoSummary) -> anyhow::Result<()>;
    /// No production reader yet: nothing displays summaries.
    #[allow(dead_code)]
    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoSummary>>;
    fn delete(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
}

pub struct SqliteVideoSummaryRepository {
    #[allow(dead_code)]
    db: Database,
}

impl SqliteVideoSummaryRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl VideoSummaryRepository for SqliteVideoSummaryRepository {
    fn save(&self, _summary: &VideoSummary) -> anyhow::Result<()> {
        Ok(())
    }

    fn find(&self, _video_id: &VideoRecordId) -> anyhow::Result<Option<VideoSummary>> {
        Ok(None)
    }

    fn delete(&self, _video_id: &VideoRecordId) -> anyhow::Result<()> {
        Ok(())
    }
}
