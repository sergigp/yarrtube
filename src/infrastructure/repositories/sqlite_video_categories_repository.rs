use crate::domain::category::CategoryId;
use crate::domain::video::VideoRecordId;
use crate::domain::video_category::VideoCategories;
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use rusqlite::{Connection, params, params_from_iter};
use std::collections::HashMap;

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
    db: Database,
}

impl SqliteVideoCategoriesRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl VideoCategoriesRepository for SqliteVideoCategoriesRepository {
    fn find_many(&self, ids: &[VideoRecordId]) -> anyhow::Result<Vec<VideoCategories>> {
        let conn = self.db.read()?;
        let placeholders = vec!["?"; ids.len()].join(", ");
        let mut stmt = conn
            .prepare(&format!(
                "SELECT video_id, category_id FROM video_categories WHERE video_id IN ({placeholders})"
            ))
            .context("failed to prepare find many video categories")?;
        let found = stmt
            .query_map(
                params_from_iter(ids.iter().map(|id| id.as_str())),
                Self::read_row,
            )
            .inspect_err(|e| tracing::error!(error = %e, "failed to find video categories"))
            .context("failed to find video categories")?
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to read video category row")?;
        let mut by_video = Self::group_by_video(found)?;
        Ok(ids
            .iter()
            .map(|id| {
                VideoCategories::new(id.clone(), by_video.remove(id.as_str()).unwrap_or_default())
            })
            .collect())
    }

    fn list_in_category(&self, _category_id: &CategoryId) -> anyhow::Result<Vec<VideoCategories>> {
        Ok(Vec::new())
    }

    fn save_many(&self, categories: &[VideoCategories]) -> anyhow::Result<()> {
        let mut conn = self.db.write()?;
        let tx = conn
            .transaction()
            .context("failed to begin video categories save")?;
        categories
            .iter()
            .try_for_each(|video_categories| Self::replace(&tx, video_categories))?;
        tx.commit()
            .context("failed to commit video categories save")
    }

    fn delete(&self, _video_id: &VideoRecordId) -> anyhow::Result<()> {
        Ok(())
    }
}

impl SqliteVideoCategoriesRepository {
    fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, String)> {
        Ok((row.get(0)?, row.get(1)?))
    }

    /// Replaces the record's rows with its set.
    fn replace(conn: &Connection, video_categories: &VideoCategories) -> anyhow::Result<()> {
        let video_id = &video_categories.video_id;
        conn.execute(
            "DELETE FROM video_categories WHERE video_id = ?1",
            params![video_id.as_str()],
        )
        .inspect_err(
            |e| tracing::error!(video_id = %video_id, error = %e, "failed to delete video categories"),
        )
        .context("failed to delete video categories")?;
        video_categories
            .category_ids
            .iter()
            .try_for_each(|category_id| {
                conn.execute(
                    "INSERT INTO video_categories (video_id, category_id) VALUES (?1, ?2)",
                    params![video_id.as_str(), category_id.as_str()],
                )
                .inspect_err(|e| tracing::error!(video_id = %video_id, error = %e, "failed to insert video category"))
                .context("failed to insert video category")
                .map(|_| ())
            })
    }

    /// Each video id's category ids, from `(video_id, category_id)` rows.
    fn group_by_video(
        rows: Vec<(String, String)>,
    ) -> anyhow::Result<HashMap<String, Vec<CategoryId>>> {
        rows.into_iter()
            .try_fold(HashMap::new(), |mut grouped, (video_id, category_id)| {
                grouped
                    .entry(video_id)
                    .or_insert_with(Vec::new)
                    .push(CategoryId::new(category_id)?);
                Ok(grouped)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;

    #[test]
    fn it_should_find_none_for_a_record_without_rows() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoriesRepository::new(db.database());

        assert_eq!(
            repository.find_many(&[record("rec1")]).unwrap(),
            vec![VideoCategories::none(record("rec1"))]
        );
    }

    #[test]
    fn it_should_replace_a_record_set_on_save() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoriesRepository::new(db.database());
        repository
            .save_many(&[
                categories("rec1", &["chess", "funny"]),
                categories("rec2", &["funny"]),
            ])
            .unwrap();

        repository
            .save_many(&[categories("rec1", &["science", "chess"])])
            .unwrap();

        assert_eq!(
            repository
                .find_many(&[record("rec1"), record("rec2")])
                .unwrap(),
            vec![
                categories("rec1", &["chess", "science"]),
                categories("rec2", &["funny"]),
            ]
        );
    }

    fn categories(video_id: &str, category_ids: &[&str]) -> VideoCategories {
        VideoCategories::new(
            record(video_id),
            category_ids
                .iter()
                .map(|id| CategoryId::new(*id).unwrap())
                .collect(),
        )
    }

    fn record(id: &str) -> VideoRecordId {
        VideoRecordId::new(id).unwrap()
    }
}
