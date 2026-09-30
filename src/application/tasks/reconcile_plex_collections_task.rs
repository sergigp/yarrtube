use crate::domain::services::{PlexCollectionReconciler, PlexCollectionReconcilerApi};
use crate::domain::task::Task;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;
use tracing::info;

/// Recurring global Plex collections convergence pass, reachable only via
/// the task queue (seeded once at daemon startup by
/// `schedule_reconcile_plex_collections_if_absent`, only when the Plex
/// integration is enabled). Like `UpdateYtdlpTask`, it always reschedules
/// its next occurrence, whether or not this pass succeeded — a failed pass
/// is retried by the next one, not by the task queue's retry/dead-letter
/// machinery.
pub struct ReconcilePlexCollectionsTask {
    reconciler: PlexCollectionReconciler,
    #[allow(dead_code)]
    task_repository: Arc<dyn TaskRepository>,
    #[allow(dead_code)]
    clock: Arc<dyn Clock>,
    #[allow(dead_code)]
    interval_seconds: i64,
}

impl ReconcilePlexCollectionsTask {
    pub fn new(
        reconciler: PlexCollectionReconciler,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
        interval_seconds: i64,
    ) -> Self {
        Self {
            reconciler,
            task_repository,
            clock,
            interval_seconds,
        }
    }
}

impl TaskHandler for ReconcilePlexCollectionsTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        Task::decode_reconcile_plex_collections_payload(payload)?;
        self.reconciler.reconcile_all()
    }
}

/// Seeds the recurring `reconcile_plex_collections` task chain to run now,
/// unless a non-terminal (`pending` or `running`) one is already scheduled —
/// a restarted daemon must not stack up additional chains alongside one that
/// already self-perpetuates forever.
pub fn schedule_reconcile_plex_collections_if_absent(
    task_repository: &Arc<dyn TaskRepository>,
    clock: &Arc<dyn Clock>,
) -> anyhow::Result<()> {
    let existing = task_repository
        .list_non_completed()?
        .into_iter()
        .find(|task| task.task_type == Task::ReconcilePlexCollections.task_type());

    match existing {
        Some(task) => info!(
            task_id = task.id,
            "recurring Plex collections reconcile task already scheduled, skipping seed"
        ),
        None => {
            let run_at = clock.now();
            task_repository.schedule(&Task::ReconcilePlexCollections, run_at)?;
            info!(run_at = %run_at, "scheduled recurring Plex collections reconcile task");
        }
    }
    Ok(())
}
