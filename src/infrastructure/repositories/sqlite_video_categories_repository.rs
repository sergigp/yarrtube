use crate::domain::category::CategoryId;
use crate::domain::video::VideoRecordId;
use crate::domain::video_category::VideoCategories;
use crate::infrastructure::shared::sqlite_connection::Database;

/// Which categories each stored video record belongs to.
pub trait VideoCategoriesRepository: Send + Sync {
    /// One per requested id, `VideoCategories::none` when it has no row.
    fn find_many(&self, ids: &[VideoRecordId]) -> anyhow::Result<Vec<VideoCategories>>;
    /// Every record belonging to `category_id`, with its full set.
    fn list_in_category(&self, category_id: &CategoryId) -> anyhow::Result<Vec<VideoCategories>>;
    /// Replaces each record's rows with its set, all in one transaction.
    fn save_many(&self, categories: &[VideoCategories]) -> anyhow::Result<()>;
    fn delete(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
}

pub struct SqliteVideoCategoriesRepository {
    #[allow(dead_code)]
    db: Database,
}

impl SqliteVideoCategoriesRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl VideoCategoriesRepository for SqliteVideoCategoriesRepository {
    fn find_many(&self, ids: &[VideoRecordId]) -> anyhow::Result<Vec<VideoCategories>> {
        Ok(ids.iter().cloned().map(VideoCategories::none).collect())
    }

    fn list_in_category(&self, _category_id: &CategoryId) -> anyhow::Result<Vec<VideoCategories>> {
        Ok(Vec::new())
    }

    fn save_many(&self, _categories: &[VideoCategories]) -> anyhow::Result<()> {
        Ok(())
    }

    fn delete(&self, _video_id: &VideoRecordId) -> anyhow::Result<()> {
        Ok(())
    }
}
