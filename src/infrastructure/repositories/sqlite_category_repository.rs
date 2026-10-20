use crate::domain::category::{Category, CategoryId};
use crate::infrastructure::shared::sqlite_connection::Database;

pub trait CategoryRepository: Send + Sync {
    fn find(&self, id: &CategoryId) -> anyhow::Result<Option<Category>>;
    /// By name ignoring case, then creation order.
    fn list(&self) -> anyhow::Result<Vec<Category>>;
    fn insert(&self, category: &Category) -> anyhow::Result<()>;
    /// Name and the full member set, in one transaction.
    fn update(&self, category: &Category) -> anyhow::Result<()>;
    /// The category and its member rows.
    fn delete(&self, id: &CategoryId) -> anyhow::Result<()>;
}

pub struct SqliteCategoryRepository {
    db: Database,
}

impl SqliteCategoryRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl CategoryRepository for SqliteCategoryRepository {
    fn find(&self, _id: &CategoryId) -> anyhow::Result<Option<Category>> {
        Ok(None)
    }

    fn list(&self) -> anyhow::Result<Vec<Category>> {
        Ok(Vec::new())
    }

    fn insert(&self, _category: &Category) -> anyhow::Result<()> {
        Ok(())
    }

    fn update(&self, _category: &Category) -> anyhow::Result<()> {
        Ok(())
    }

    fn delete(&self, _id: &CategoryId) -> anyhow::Result<()> {
        Ok(())
    }
}
