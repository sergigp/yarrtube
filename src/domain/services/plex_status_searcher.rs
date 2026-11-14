use crate::domain::plex_status::PlexStatus;
use crate::infrastructure::repositories::sqlite_plex_sync_run_repository::PlexSyncRunRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use std::sync::Arc;

/// Reports how the Plex integration is doing from what its syncs recorded,
/// without calling Plex.
#[derive(Clone)]
pub struct PlexStatusSearcher {
    #[allow(dead_code)]
    configured: bool,
    #[allow(dead_code)]
    plex_sync_run_repository: Arc<dyn PlexSyncRunRepository>,
    #[allow(dead_code)]
    task_repository: Arc<dyn TaskRepository>,
}

impl PlexStatusSearcher {
    pub fn new(
        configured: bool,
        plex_sync_run_repository: Arc<dyn PlexSyncRunRepository>,
        task_repository: Arc<dyn TaskRepository>,
    ) -> Self {
        Self {
            configured,
            plex_sync_run_repository,
            task_repository,
        }
    }
}

pub trait PlexStatusSearcherApi: Send + Sync {
    /// Whether Plex is configured and, when it is, the connection as the
    /// latest sync saw it and where each sync stands.
    fn get(&self) -> anyhow::Result<PlexStatus>;
}

impl PlexStatusSearcherApi for PlexStatusSearcher {
    fn get(&self) -> anyhow::Result<PlexStatus> {
        Ok(PlexStatus {
            configured: false,
            connection: None,
            syncs: Vec::new(),
        })
    }
}
