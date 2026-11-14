use crate::domain::plex_status::PlexSyncRun;
use crate::infrastructure::shared::sqlite_connection::Database;

/// The latest finished run of each Plex sync.
pub trait PlexSyncRunRepository: Send + Sync {
    /// Replaces the previous run of the same kind.
    fn save(&self, run: &PlexSyncRun) -> anyhow::Result<()>;
    /// The latest run of every kind that has finished one.
    fn list(&self) -> anyhow::Result<Vec<PlexSyncRun>>;
}

pub struct SqlitePlexSyncRunRepository {
    #[allow(dead_code)]
    db: Database,
}

impl SqlitePlexSyncRunRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl PlexSyncRunRepository for SqlitePlexSyncRunRepository {
    fn save(&self, _run: &PlexSyncRun) -> anyhow::Result<()> {
        Ok(())
    }

    fn list(&self) -> anyhow::Result<Vec<PlexSyncRun>> {
        Ok(Vec::new())
    }
}
