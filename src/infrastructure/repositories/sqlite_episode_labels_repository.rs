use crate::domain::plex::PlexLabels;
use crate::domain::plex_label::EpisodeLabels;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params, params_from_iter};

const COLUMNS: &str = "video_id, dirty, rating_key, section_id, labels, checked_at";

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
    fn find_many(&self, ids: &[VideoRecordId]) -> anyhow::Result<Vec<EpisodeLabels>> {
        let placeholders = vec!["?"; ids.len()].join(", ");
        let found = self.query(
            &format!("SELECT {COLUMNS} FROM episode_labels WHERE video_id IN ({placeholders})"),
            ids.iter().map(|id| id.as_str()),
        )?;
        Ok(ids
            .iter()
            .filter_map(|id| found.iter().find(|row| &row.video_id == id).cloned())
            .collect())
    }

    fn list_dirty(&self, limit: usize) -> anyhow::Result<Vec<EpisodeLabels>> {
        self.query(
            &format!(
                "SELECT {COLUMNS} FROM episode_labels WHERE dirty = 1
                 ORDER BY checked_at IS NOT NULL, checked_at, video_id LIMIT {limit}"
            ),
            [],
        )
    }

    fn list_synced(&self) -> anyhow::Result<Vec<EpisodeLabels>> {
        Ok(Vec::new())
    }

    fn save_many(&self, rows: &[EpisodeLabels]) -> anyhow::Result<()> {
        let mut conn = self.db.write()?;
        let tx = conn
            .transaction()
            .context("failed to begin episode labels save")?;
        rows.iter().try_for_each(|row| Self::upsert(&tx, row))?;
        tx.commit().context("failed to commit episode labels save")
    }

    fn delete(&self, _video_id: &VideoRecordId) -> anyhow::Result<()> {
        Ok(())
    }
}

impl SqliteEpisodeLabelsRepository {
    fn query<'a>(
        &self,
        sql: &str,
        params: impl IntoIterator<Item = &'a str>,
    ) -> anyhow::Result<Vec<EpisodeLabels>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(sql)
            .context("failed to prepare episode labels query")?;
        let rows = stmt
            .query_map(params_from_iter(params), Self::read_row)
            .inspect_err(|e| tracing::error!(error = %e, "failed to query episode labels"))
            .context("failed to query episode labels")?
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to read episode labels row")?;
        rows.into_iter().map(Self::to_episode_labels).collect()
    }

    fn upsert(conn: &Connection, row: &EpisodeLabels) -> anyhow::Result<()> {
        conn.execute(
            &format!("INSERT OR REPLACE INTO episode_labels ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"),
            params![
                row.video_id.as_str(),
                row.dirty,
                row.rating_key,
                row.section_id,
                serde_json::to_string(row.labels.names())?,
                row.checked_at.map(|at| at.to_rfc3339()),
            ],
        )
        .inspect_err(|e| tracing::error!(video_id = %row.video_id, error = %e, "failed to save episode labels"))
        .context("failed to save episode labels")
        .map(drop)
    }

    fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawRow> {
        Ok(RawRow {
            video_id: row.get(0)?,
            dirty: row.get(1)?,
            rating_key: row.get(2)?,
            section_id: row.get(3)?,
            labels: row.get(4)?,
            checked_at: row.get(5)?,
        })
    }

    fn to_episode_labels(row: RawRow) -> anyhow::Result<EpisodeLabels> {
        Ok(EpisodeLabels {
            video_id: VideoRecordId::new(row.video_id)?,
            dirty: row.dirty,
            rating_key: row.rating_key,
            section_id: row.section_id,
            labels: PlexLabels::new(
                serde_json::from_str(&row.labels).context("invalid stored episode labels")?,
            ),
            checked_at: row
                .checked_at
                .map(|at| DateTime::parse_from_rfc3339(&at).map(|at| at.with_timezone(&Utc)))
                .transpose()
                .context("invalid stored episode labels check time")?,
        })
    }
}

/// An `episode_labels` row as stored.
struct RawRow {
    video_id: String,
    dirty: bool,
    rating_key: Option<String>,
    section_id: Option<String>,
    labels: String,
    checked_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plex::PlexLabels;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use chrono::{DateTime, Utc};

    #[test]
    fn it_should_round_trip_an_episode_labels_row() {
        let db = TestDatabase::new();
        let repository = SqliteEpisodeLabelsRepository::new(db.database());
        let synced = EpisodeLabels {
            dirty: false,
            rating_key: Some("101".to_string()),
            section_id: Some("19".to_string()),
            labels: PlexLabels::new(vec!["Chess".to_string(), "Música".to_string()]),
            checked_at: Some(at("2026-10-29T10:00:00Z")),
            ..pending("rec1")
        };
        repository
            .save_many(&[synced.clone(), pending("rec2")])
            .unwrap();

        let found = repository
            .find_many(&[record("rec1"), record("rec2"), record("rec3")])
            .unwrap();

        assert_eq!(found, vec![synced, pending("rec2")]);
    }

    #[test]
    fn it_should_list_dirty_rows_least_recently_checked_first_up_to_a_limit() {
        let db = TestDatabase::new();
        let repository = SqliteEpisodeLabelsRepository::new(db.database());
        repository
            .save_many(&[
                checked("rec1", "2026-10-29T10:00:00Z"),
                pending("rec2"),
                EpisodeLabels {
                    dirty: false,
                    ..pending("rec3")
                },
                checked("rec4", "2026-10-29T09:00:00Z"),
                checked("rec5", "2026-10-29T11:00:00Z"),
            ])
            .unwrap();

        let dirty = repository.list_dirty(3).unwrap();

        assert_eq!(
            dirty,
            vec![
                pending("rec2"),
                checked("rec4", "2026-10-29T09:00:00Z"),
                checked("rec1", "2026-10-29T10:00:00Z"),
            ]
        );
    }

    fn checked(id: &str, at_time: &str) -> EpisodeLabels {
        EpisodeLabels {
            checked_at: Some(at(at_time)),
            ..pending(id)
        }
    }

    fn pending(id: &str) -> EpisodeLabels {
        EpisodeLabels {
            video_id: record(id),
            dirty: true,
            rating_key: None,
            section_id: None,
            labels: PlexLabels::none(),
            checked_at: None,
        }
    }

    fn record(id: &str) -> VideoRecordId {
        VideoRecordId::new(id.to_string()).unwrap()
    }

    fn at(timestamp: &str) -> DateTime<Utc> {
        timestamp.parse().unwrap()
    }
}
