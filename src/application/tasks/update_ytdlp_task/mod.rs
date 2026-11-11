use crate::domain::task::{Task, TaskOutcome};
use crate::infrastructure::client::ytdlp_updater::YtdlpUpdater;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

/// How often the recurring self-update task reschedules itself.
pub const UPDATE_INTERVAL_SECONDS: i64 = 3600;

/// Recurring hourly `yt-dlp` self-update, reachable only via the task queue
/// (seeded once at daemon startup by `schedule_update_ytdlp_if_absent`).
/// Always reschedules its next occurrence, whether or not this attempt
/// succeeded — a failed update is not a permanent failure of a unit of work,
/// unlike `download_video`, so this never goes through
/// `ScheduledTask::fail`/dead-letter (see design.md's "Recurring self-update
/// task" decision).
pub struct UpdateYtdlpTask {
    updater: Arc<dyn YtdlpUpdater>,
    ytdlp_path: PathBuf,
    /// Off when running against a stand-in `yt-dlp`, which an update would
    /// replace with the real one.
    self_update_enabled: bool,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl UpdateYtdlpTask {
    pub fn new(
        updater: Arc<dyn YtdlpUpdater>,
        ytdlp_path: PathBuf,
        self_update_enabled: bool,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            updater,
            ytdlp_path,
            self_update_enabled,
            task_repository,
            clock,
        }
    }
}

impl TaskHandler for UpdateYtdlpTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::UpdateYtdlp = task else {
            return Err(unexpected_task(task));
        };

        if self.self_update_enabled {
            match self.updater.update(&self.ytdlp_path) {
                Ok(()) => info!("yt-dlp self-update succeeded"),
                Err(e) => error!(error = %e, "yt-dlp self-update failed"),
            }
        } else {
            info!("yt-dlp self-update is off, leaving yt-dlp as it is");
        }

        let next_run_at = self.clock.now() + chrono::Duration::seconds(UPDATE_INTERVAL_SECONDS);
        self.task_repository
            .schedule(&Task::UpdateYtdlp, next_run_at)?;
        info!(next_run_at = %next_run_at, "scheduled next yt-dlp self-update");

        Ok(TaskOutcome::Done)
    }
}

/// Seeds the recurring `update_ytdlp` task chain at `run_at`, unless a
/// non-terminal (`pending` or `running`) one is already scheduled — a
/// restarted daemon must not stack up additional hourly self-update chains
/// alongside one that already self-perpetuates forever.
pub fn schedule_update_ytdlp_if_absent(
    task_repository: &dyn TaskRepository,
    run_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    let existing = task_repository
        .list_non_completed()?
        .into_iter()
        .find(|task| task.task == Task::UpdateYtdlp);

    match existing {
        Some(task) => info!(
            task_id = task.id,
            "recurring yt-dlp self-update task already scheduled, skipping seed"
        ),
        None => {
            task_repository.schedule(&Task::UpdateYtdlp, run_at)?;
            info!(run_at = %run_at, "scheduled recurring yt-dlp self-update task");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
