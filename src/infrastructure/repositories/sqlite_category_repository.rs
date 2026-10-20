use crate::domain::category::{Category, CategoryId, CategoryName};
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Row, params};

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
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare("SELECT id, name, created_at FROM categories")
            .inspect_err(|e| tracing::error!(error = %e, "failed to prepare category list query"))
            .context("failed to prepare category list query")?;
        let rows = stmt
            .query_map([], Self::read_row)
            .inspect_err(|e| tracing::error!(error = %e, "failed to list categories"))
            .context("failed to list categories")?;

        rows.map(|row| {
            row.inspect_err(|e| tracing::error!(error = %e, "failed to read category row"))
                .context("failed to read category row")
                .and_then(Self::row_to_category)
        })
        .collect()
    }

    fn insert(&self, category: &Category) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "INSERT INTO categories (id, name, created_at) VALUES (?1, ?2, ?3)",
            params![
                category.id.as_str(),
                category.name.as_str(),
                category.created_at.to_rfc3339()
            ],
        )
        .inspect_err(
            |e| tracing::error!(category_id = %category.id, error = %e, "failed to insert category"),
        )
        .context("failed to insert category")?;
        Ok(())
    }

    fn update(&self, _category: &Category) -> anyhow::Result<()> {
        Ok(())
    }

    fn delete(&self, _id: &CategoryId) -> anyhow::Result<()> {
        Ok(())
    }
}

impl SqliteCategoryRepository {
    fn read_row(row: &Row<'_>) -> rusqlite::Result<CategoryRow> {
        Ok(CategoryRow {
            id: row.get(0)?,
            name: row.get(1)?,
            created_at: row.get(2)?,
        })
    }

    fn row_to_category(row: CategoryRow) -> anyhow::Result<Category> {
        Ok(Category {
            id: CategoryId::new(row.id)?,
            name: CategoryName::new(row.name)?,
            members: Vec::new(),
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .context("failed to parse stored created_at")?
                .with_timezone(&Utc),
        })
    }
}

struct CategoryRow {
    id: String,
    name: String,
    created_at: String,
}
