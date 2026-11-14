use crate::domain::plex_status::{PlexSyncKind, PlexSyncOutcome, PlexSyncRun};
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::params;

/// The latest finished run of each Plex sync.
pub trait PlexSyncRunRepository: Send + Sync {
    /// Replaces the previous run of the same kind.
    fn save(&self, run: &PlexSyncRun) -> anyhow::Result<()>;
    /// The latest run of every kind that has finished one.
    fn list(&self) -> anyhow::Result<Vec<PlexSyncRun>>;
}

pub struct SqlitePlexSyncRunRepository {
    db: Database,
}

impl SqlitePlexSyncRunRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

/// A stored row: kind, finished at, error, version and libraries (JSON).
type StoredRun = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

impl PlexSyncRunRepository for SqlitePlexSyncRunRepository {
    fn save(&self, run: &PlexSyncRun) -> anyhow::Result<()> {
        let (error, version, libraries) = match &run.outcome {
            PlexSyncOutcome::Reached { version, libraries } => (
                None,
                Some(version.clone()),
                Some(serde_json::to_string(libraries)?),
            ),
            PlexSyncOutcome::Failed { error } => (Some(error.clone()), None, None),
        };
        let conn = self.db.write()?;
        conn.execute(
            "INSERT OR REPLACE INTO plex_sync_runs (kind, finished_at, error, version, libraries)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                run.kind.as_str(),
                run.finished_at.to_rfc3339(),
                error,
                version,
                libraries
            ],
        )
        .inspect_err(|e| tracing::error!(kind = run.kind.as_str(), error = %e, "failed to save Plex sync run"))
        .context("failed to save Plex sync run")
        .map(drop)
    }

    fn list(&self) -> anyhow::Result<Vec<PlexSyncRun>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(
                "SELECT kind, finished_at, error, version, libraries FROM plex_sync_runs
                 ORDER BY kind",
            )
            .context("failed to prepare Plex sync runs query")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .inspect_err(|e| tracing::error!(error = %e, "failed to list Plex sync runs"))
            .context("failed to list Plex sync runs")?
            .collect::<rusqlite::Result<Vec<StoredRun>>>()
            .context("failed to read Plex sync run row")?;
        rows.into_iter().filter_map(Self::to_run).collect()
    }
}

impl SqlitePlexSyncRunRepository {
    /// The run a row holds; `None` for a kind this version doesn't know.
    fn to_run(row: StoredRun) -> Option<anyhow::Result<PlexSyncRun>> {
        let (kind, finished_at, error, version, libraries) = row;
        let kind = PlexSyncKind::ALL
            .into_iter()
            .find(|known| known.as_str() == kind)?;
        Some(
            Self::outcome(error, version, libraries).and_then(|outcome| {
                Ok(PlexSyncRun {
                    kind,
                    finished_at: DateTime::parse_from_rfc3339(&finished_at)
                        .context("invalid Plex sync run finish time")?
                        .with_timezone(&Utc),
                    outcome,
                })
            }),
        )
    }

    fn outcome(
        error: Option<String>,
        version: Option<String>,
        libraries: Option<String>,
    ) -> anyhow::Result<PlexSyncOutcome> {
        Ok(match error {
            Some(error) => PlexSyncOutcome::Failed { error },
            None => PlexSyncOutcome::Reached {
                version: version.unwrap_or_default(),
                libraries: libraries
                    .map(|json| serde_json::from_str(&json))
                    .transpose()
                    .context("invalid Plex sync run libraries")?
                    .unwrap_or_default(),
            },
        })
    }
}
