use crate::domain::task::Task;
use crate::infrastructure::client::ytdlp_updater::YtdlpUpdater;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use crate::infrastructure::shared::system_clock::Clock;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

/// How often the recurring self-update task reschedules itself.
pub const UPDATE_INTERVAL_SECONDS: i64 = 3600;

/// Recurring hourly `yt-dlp` self-update, reachable only via the task queue
/// (seeded once at daemon startup, see `serve.rs`). Always reschedules its
/// next occurrence, whether or not this attempt succeeded — a failed update
/// is not a permanent failure of a unit of work, unlike `download_video`, so
/// this never goes through `ScheduledTask::fail`/dead-letter (see
/// design.md's "Recurring self-update task" decision).
pub struct UpdateYtdlpTask {
    updater: Arc<dyn YtdlpUpdater>,
    ytdlp_path: PathBuf,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl UpdateYtdlpTask {
    pub fn new(
        updater: Arc<dyn YtdlpUpdater>,
        ytdlp_path: PathBuf,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            updater,
            ytdlp_path,
            task_repository,
            clock,
        }
    }
}

impl TaskHandler for UpdateYtdlpTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        Task::decode_update_ytdlp_payload(payload)?;

        match self.updater.update(&self.ytdlp_path) {
            Ok(()) => info!("yt-dlp self-update succeeded"),
            Err(e) => error!(error = %e, "yt-dlp self-update failed"),
        }

        let next_run_at = self.clock.now() + chrono::Duration::seconds(UPDATE_INTERVAL_SECONDS);
        self.task_repository
            .schedule(&Task::UpdateYtdlp, next_run_at)?;
        info!(next_run_at = %next_run_at, "scheduled next yt-dlp self-update");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::infrastructure::client::ytdlp_updater::FakeYtdlpUpdater;
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    #[test]
    fn it_should_reschedule_after_success() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let task = UpdateYtdlpTask::new(
            Arc::new(FakeYtdlpUpdater { succeeds: true }),
            PathBuf::from("/app/bin/yt-dlp"),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::UpdateYtdlp,
                fixed_timestamp() + chrono::Duration::seconds(UPDATE_INTERVAL_SECONDS),
            )]
        );
    }

    #[test]
    fn it_should_reschedule_after_failure() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let task = UpdateYtdlpTask::new(
            Arc::new(FakeYtdlpUpdater { succeeds: false }),
            PathBuf::from("/app/bin/yt-dlp"),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::UpdateYtdlp,
                fixed_timestamp() + chrono::Duration::seconds(UPDATE_INTERVAL_SECONDS),
            )]
        );
    }

    fn pending_task(id: i64, task: &Task, run_at: DateTime<Utc>) -> ScheduledTask {
        ScheduledTask {
            id,
            task_type: task.task_type().to_string(),
            payload: task.payload().to_string(),
            status: TaskStatus::Pending,
            retries: 0,
            run_at,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn run(task: &UpdateYtdlpTask, payload: &str) -> Result<(), String> {
        task.handle(payload, false).map_err(|e| e.to_string())
    }
}
