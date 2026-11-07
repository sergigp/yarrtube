use crate::domain::category::CategoryId;
use crate::domain::video::VideoRecordId;
use crate::domain::video_category::VideoCategoryVerdict;
use crate::infrastructure::shared::sqlite_connection::Database;

/// Each (video, category) pair's verdict, at most one per pair.
pub trait VideoCategoryVerdictRepository: Send + Sync {
    /// Every verdict of `video_ids`.
    fn find_many(&self, video_ids: &[VideoRecordId]) -> anyhow::Result<Vec<VideoCategoryVerdict>>;
    /// Replaces the pair's verdict.
    fn save(&self, verdict: &VideoCategoryVerdict) -> anyhow::Result<()>;
    fn delete_for_video(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
    fn delete_for_category(&self, category_id: &CategoryId) -> anyhow::Result<()>;
    /// Every `unavailable` verdict.
    fn delete_unavailable(&self) -> anyhow::Result<()>;
    /// `category_id`'s `unavailable` verdicts.
    fn delete_unavailable_for_category(&self, category_id: &CategoryId) -> anyhow::Result<()>;
}

pub struct SqliteVideoCategoryVerdictRepository {
    #[allow(dead_code)]
    db: Database,
}

impl SqliteVideoCategoryVerdictRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl VideoCategoryVerdictRepository for SqliteVideoCategoryVerdictRepository {
    fn find_many(&self, _video_ids: &[VideoRecordId]) -> anyhow::Result<Vec<VideoCategoryVerdict>> {
        Ok(Vec::new())
    }

    fn save(&self, _verdict: &VideoCategoryVerdict) -> anyhow::Result<()> {
        Ok(())
    }

    fn delete_for_video(&self, _video_id: &VideoRecordId) -> anyhow::Result<()> {
        Ok(())
    }

    fn delete_for_category(&self, _category_id: &CategoryId) -> anyhow::Result<()> {
        Ok(())
    }

    fn delete_unavailable(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn delete_unavailable_for_category(&self, _category_id: &CategoryId) -> anyhow::Result<()> {
        Ok(())
    }
}
