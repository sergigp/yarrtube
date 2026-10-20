use crate::domain::category::{Category, CategoryId, CategoryMember, CategoryName};
use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params};

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
    fn find(&self, id: &CategoryId) -> anyhow::Result<Option<Category>> {
        let conn = self.db.read()?;
        conn.query_row(
            "SELECT id, name, created_at FROM categories WHERE id = ?1",
            params![id.as_str()],
            Self::read_row,
        )
        .optional()
        .inspect_err(|e| tracing::error!(category_id = %id, error = %e, "failed to query category"))
        .context("failed to query category")?
        .map(|row| Self::row_to_category(&conn, row))
        .transpose()
    }

    fn list(&self) -> anyhow::Result<Vec<Category>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, created_at FROM categories ORDER BY name COLLATE NOCASE, rowid",
            )
            .inspect_err(|e| tracing::error!(error = %e, "failed to prepare category list query"))
            .context("failed to prepare category list query")?;
        let rows = stmt
            .query_map([], Self::read_row)
            .inspect_err(|e| tracing::error!(error = %e, "failed to list categories"))
            .context("failed to list categories")?;

        rows.map(|row| {
            row.inspect_err(|e| tracing::error!(error = %e, "failed to read category row"))
                .context("failed to read category row")
                .and_then(|row| Self::row_to_category(&conn, row))
        })
        .collect()
    }

    fn insert(&self, category: &Category) -> anyhow::Result<()> {
        let mut conn = self.db.write()?;
        let tx = conn
            .transaction()
            .context("failed to begin category insert")?;
        tx.execute(
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
        Self::insert_members(&tx, category)?;
        tx.commit().context("failed to commit category insert")
    }

    fn update(&self, category: &Category) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "UPDATE categories SET name = ?2 WHERE id = ?1",
            params![category.id.as_str(), category.name.as_str()],
        )
        .inspect_err(
            |e| tracing::error!(category_id = %category.id, error = %e, "failed to update category"),
        )
        .context("failed to update category")?;
        Ok(())
    }

    fn delete(&self, id: &CategoryId) -> anyhow::Result<()> {
        let mut conn = self.db.write()?;
        let tx = conn
            .transaction()
            .context("failed to begin category delete")?;
        Self::delete_members(&tx, id)?;
        tx.execute("DELETE FROM categories WHERE id = ?1", params![id.as_str()])
            .inspect_err(
                |e| tracing::error!(category_id = %id, error = %e, "failed to delete category"),
            )
            .context("failed to delete category")?;
        tx.commit().context("failed to commit category delete")
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

    fn insert_members(conn: &Connection, category: &Category) -> anyhow::Result<()> {
        category.members.iter().try_for_each(|member| {
            let (kind, id) = Self::member_columns(member);
            conn.execute(
                "INSERT INTO category_members (category_id, member_kind, member_id) VALUES (?1, ?2, ?3)",
                params![category.id.as_str(), kind, id],
            )
            .inspect_err(|e| tracing::error!(category_id = %category.id, error = %e, "failed to insert category member"))
            .context("failed to insert category member")
            .map(|_| ())
        })
    }

    fn delete_members(conn: &Connection, id: &CategoryId) -> anyhow::Result<()> {
        conn.execute(
            "DELETE FROM category_members WHERE category_id = ?1",
            params![id.as_str()],
        )
        .inspect_err(
            |e| tracing::error!(category_id = %id, error = %e, "failed to delete category members"),
        )
        .context("failed to delete category members")?;
        Ok(())
    }

    fn members(conn: &Connection, id: &CategoryId) -> anyhow::Result<Vec<CategoryMember>> {
        let mut stmt = conn
            .prepare(
                "SELECT member_kind, member_id FROM category_members WHERE category_id = ?1 ORDER BY rowid",
            )
            .context("failed to prepare category members query")?;
        let rows = stmt
            .query_map(params![id.as_str()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .inspect_err(|e| tracing::error!(category_id = %id, error = %e, "failed to list category members"))
            .context("failed to list category members")?;
        rows.map(|row| {
            row.context("failed to read category member row")
                .and_then(|(kind, member_id)| Self::row_to_member(&kind, member_id))
        })
        .collect()
    }

    fn member_columns(member: &CategoryMember) -> (&'static str, &str) {
        match member {
            CategoryMember::Channel(handle) => (CHANNEL_KIND, handle.as_str()),
            CategoryMember::Playlist(id) => (PLAYLIST_KIND, id.as_str()),
        }
    }

    fn row_to_member(kind: &str, id: String) -> anyhow::Result<CategoryMember> {
        match kind {
            CHANNEL_KIND => Ok(CategoryMember::Channel(ChannelHandle::new(id)?)),
            PLAYLIST_KIND => Ok(CategoryMember::Playlist(PlaylistId::new(id)?)),
            other => Err(anyhow::anyhow!("unknown category member kind \"{other}\"")),
        }
    }

    fn row_to_category(conn: &Connection, row: CategoryRow) -> anyhow::Result<Category> {
        let id = CategoryId::new(row.id)?;
        Ok(Category {
            members: Self::members(conn, &id)?,
            id,
            name: CategoryName::new(row.name)?,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .context("failed to parse stored created_at")?
                .with_timezone(&Utc),
        })
    }
}

const CHANNEL_KIND: &str = "channel";
const PLAYLIST_KIND: &str = "playlist";

struct CategoryRow {
    id: String,
    name: String,
    created_at: String,
}
