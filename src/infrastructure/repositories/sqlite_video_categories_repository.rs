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

    fn list_in_category(&self, category_id: &CategoryId) -> anyhow::Result<Vec<VideoCategories>> {
        let video_ids = self.video_ids_in_category(category_id)?;
        self.find_many(&video_ids)
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

    fn delete(&self, video_id: &VideoRecordId) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        Self::delete_rows(&conn, video_id)
    }
}

impl SqliteVideoCategoriesRepository {
    /// The records holding `category_id`, by id.
    fn video_ids_in_category(
        &self,
        category_id: &CategoryId,
    ) -> anyhow::Result<Vec<VideoRecordId>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(
                "SELECT video_id FROM video_categories WHERE category_id = ?1 ORDER BY video_id",
            )
            .context("failed to prepare videos in category query")?;
        let ids = stmt
            .query_map(params![category_id.as_str()], |row| row.get::<_, String>(0))
            .inspect_err(|e| tracing::error!(category_id = %category_id, error = %e, "failed to list videos in category"))
            .context("failed to list videos in category")?
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to read video category row")?;
        ids.into_iter()
            .map(|id| VideoRecordId::new(id).map_err(anyhow::Error::from))
            .collect()
    }

    fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, String)> {
        Ok((row.get(0)?, row.get(1)?))
    }

    /// Replaces the record's rows with its set.
    fn replace(conn: &Connection, video_categories: &VideoCategories) -> anyhow::Result<()> {
        let video_id = &video_categories.video_id;
        Self::delete_rows(conn, video_id)?;
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

    fn delete_rows(conn: &Connection, video_id: &VideoRecordId) -> anyhow::Result<()> {
        conn.execute(
            "DELETE FROM video_categories WHERE video_id = ?1",
            params![video_id.as_str()],
        )
        .inspect_err(
            |e| tracing::error!(video_id = %video_id, error = %e, "failed to delete video categories"),
        )
        .context("failed to delete video categories")?;
        Ok(())
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

    #[test]
    fn it_should_delete_all_rows_when_saving_an_empty_set() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoriesRepository::new(db.database());
        repository
            .save_many(&[categories("rec1", &["chess", "funny"])])
            .unwrap();

        repository
            .save_many(&[VideoCategories::none(record("rec1"))])
            .unwrap();

        assert_eq!(
            repository.find_many(&[record("rec1")]).unwrap(),
            vec![VideoCategories::none(record("rec1"))]
        );
    }

    #[test]
    fn it_should_list_records_in_a_category_with_their_full_sets() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoriesRepository::new(db.database());
        repository
            .save_many(&[
                categories("rec2", &["chess"]),
                categories("rec1", &["funny", "chess"]),
                categories("rec3", &["funny"]),
            ])
            .unwrap();

        assert_eq!(
            repository.list_in_category(&category("chess")).unwrap(),
            vec![
                categories("rec1", &["chess", "funny"]),
                categories("rec2", &["chess"]),
            ]
        );
    }

    #[test]
    fn it_should_delete_a_record_rows() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoriesRepository::new(db.database());
        repository
            .save_many(&[
                categories("rec1", &["chess", "funny"]),
                categories("rec2", &["chess"]),
            ])
            .unwrap();

        repository.delete(&record("rec1")).unwrap();

        assert_eq!(
            repository
                .find_many(&[record("rec1"), record("rec2")])
                .unwrap(),
            vec![
                VideoCategories::none(record("rec1")),
                categories("rec2", &["chess"]),
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

    fn category(id: &str) -> CategoryId {
        CategoryId::new(id).unwrap()
    }

    fn record(id: &str) -> VideoRecordId {
        VideoRecordId::new(id).unwrap()
    }
}
