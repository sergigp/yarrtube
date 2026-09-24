use crate::domain::task::TaskFailureOutcome;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

pub type HandlerRegistry = HashMap<String, Arc<dyn TaskHandler>>;

/// Task-agnostic: polls `TaskRepository` for eligible tasks and dispatches
/// each to the single handler registered for its type.
pub struct TaskExecutor {
    repository: Arc<dyn TaskRepository>,
    handlers: HandlerRegistry,
    clock: Arc<dyn Clock>,
    base_retry_delay_seconds: i64,
}

impl TaskExecutor {
    pub fn new(
        repository: Arc<dyn TaskRepository>,
        handlers: HandlerRegistry,
        clock: Arc<dyn Clock>,
        base_retry_delay_seconds: i64,
    ) -> Self {
        Self {
            repository,
            handlers,
            clock,
            base_retry_delay_seconds,
        }
    }

    pub fn poll_once(&self) -> anyhow::Result<()> {
        for task in self.repository.list_eligible()? {
            let id = task.id;
            let task_type = task.task_type.clone();
            let payload = task.payload.clone();
            let running = task.start(self.clock.now());
            self.repository.update(&running)?;
            let is_last_attempt = running.is_last_attempt();
            info!(task_id = id, task_type = %task_type, "dispatching task");

            let outcome = match self.handlers.get(&task_type) {
                Some(handler) => handler.handle(&payload, is_last_attempt),
                None => Err(anyhow::anyhow!(
                    "no handler registered for task type '{}'",
                    task_type
                )),
            };

            match outcome {
                Ok(()) => {
                    self.repository.delete(id)?;
                    info!(task_id = id, "task done");
                }
                Err(e) => {
                    let error = e.to_string();
                    match running.fail(
                        error.clone(),
                        self.clock.now(),
                        self.base_retry_delay_seconds,
                    ) {
                        TaskFailureOutcome::Retry(retried) => {
                            warn!(
                                task_id = id,
                                retries = retried.retries,
                                run_at = %retried.run_at,
                                error,
                                "task failed, retrying"
                            );
                            self.repository.update(&retried)?;
                        }
                        TaskFailureOutcome::DeadLetter(dead) => {
                            error!(
                                task_id = id,
                                retries = dead.retries,
                                error,
                                "task failed permanently"
                            );
                            self.repository.dead_letter(&dead)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Recovers every task left `running` by a previous, interrupted
    /// process, applying the same failed-attempt handling as a dispatch
    /// failure to each. Meant to be called once at startup.
    pub fn recover_stuck_tasks(&self) -> anyhow::Result<()> {
        for task in self.repository.list_running()? {
            let id = task.id;
            warn!(
                task_id = id,
                "recovering task left running after an unclean shutdown"
            );
            match task.fail(
                "recovered as a failed attempt after an unclean shutdown",
                self.clock.now(),
                self.base_retry_delay_seconds,
            ) {
                TaskFailureOutcome::Retry(retried) => self.repository.update(&retried)?,
                TaskFailureOutcome::DeadLetter(dead) => self.repository.dead_letter(&dead)?,
            }
        }
        Ok(())
    }

    /// Polls forever on `interval`, matching `heartbeat_loop`'s shape. Meant
    /// to be handed to `tokio::spawn` by the composition root.
    pub async fn run(self: Arc<Self>, interval: Duration) {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let executor = self.clone();
            match tokio::task::spawn_blocking(move || executor.poll_once()).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => error!(error = %e, "poll failed"),
                Err(e) => error!(error = %e, "poll task panicked"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::{DeadLetteredTask, ScheduledTask, Task, TaskStatus};
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

    #[test]
    fn it_should_dispatch_an_eligible_task_and_delete_it_on_success() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(FakeHandler::succeeding());
        task_repository.schedule(&task(), now()).unwrap();
        let executor = TaskExecutor::new(
            task_repository.clone(),
            registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = executor.poll_once();

        assert!(result.is_ok());
        assert_eq!(
            *handler.received_is_last_attempt.lock().unwrap(),
            vec![false]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            Vec::<ScheduledTask>::new()
        );
        assert_eq!(
            task_repository.list_dead_lettered().unwrap(),
            Vec::<DeadLetteredTask>::new()
        );
    }

    #[test]
    fn it_should_retry_a_task_whose_handler_fails() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(FakeHandler::failing());
        task_repository.schedule(&task(), now()).unwrap();
        let executor = TaskExecutor::new(
            task_repository.clone(),
            registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = executor.poll_once();

        assert!(result.is_ok());
        assert_eq!(
            *handler.received_is_last_attempt.lock().unwrap(),
            vec![false]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![ScheduledTask {
                retries: 1,
                run_at: now() + chrono::Duration::seconds(450),
                last_error: Some("handler failed".to_string()),
                ..pending_task(0)
            }]
        );
        assert_eq!(
            task_repository.list_dead_lettered().unwrap(),
            Vec::<DeadLetteredTask>::new()
        );
    }

    #[test]
    fn it_should_dead_letter_a_task_whose_handler_fails_on_the_fifth_attempt() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(FakeHandler::failing());
        task_repository.schedule(&task(), now()).unwrap();
        task_repository.update(&pending_task(4)).unwrap();
        let executor = TaskExecutor::new(
            task_repository.clone(),
            registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = executor.poll_once();

        assert!(result.is_ok());
        assert_eq!(
            *handler.received_is_last_attempt.lock().unwrap(),
            vec![true]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            Vec::<ScheduledTask>::new()
        );
        assert_eq!(
            task_repository.list_dead_lettered().unwrap(),
            vec![dead_lettered_task("handler failed")]
        );
    }

    #[test]
    fn it_should_tell_the_handler_this_is_not_the_last_attempt_when_retries_remain() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(FakeHandler::succeeding());
        task_repository.schedule(&task(), now()).unwrap();
        task_repository.update(&pending_task(3)).unwrap();
        let executor = TaskExecutor::new(
            task_repository.clone(),
            registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = executor.poll_once();

        assert!(result.is_ok());
        assert_eq!(
            *handler.received_is_last_attempt.lock().unwrap(),
            vec![false]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            Vec::<ScheduledTask>::new()
        );
    }

    #[test]
    fn it_should_tell_the_handler_this_is_the_last_attempt_when_no_retries_remain() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(FakeHandler::succeeding());
        task_repository.schedule(&task(), now()).unwrap();
        task_repository.update(&pending_task(4)).unwrap();
        let executor = TaskExecutor::new(
            task_repository.clone(),
            registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = executor.poll_once();

        assert!(result.is_ok());
        assert_eq!(
            *handler.received_is_last_attempt.lock().unwrap(),
            vec![true]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            Vec::<ScheduledTask>::new()
        );
    }

    #[test]
    fn it_should_retry_a_task_recovered_as_running_from_a_previous_process() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(FakeHandler::succeeding());
        task_repository.schedule(&task(), now()).unwrap();
        task_repository
            .update(&pending_task(0).start(now()))
            .unwrap();
        let executor = TaskExecutor::new(
            task_repository.clone(),
            registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = executor.recover_stuck_tasks();

        assert!(result.is_ok());
        assert_eq!(
            *handler.received_is_last_attempt.lock().unwrap(),
            Vec::<bool>::new()
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![ScheduledTask {
                retries: 1,
                run_at: now() + chrono::Duration::seconds(450),
                last_error: Some(
                    "recovered as a failed attempt after an unclean shutdown".to_string()
                ),
                ..pending_task(0)
            }]
        );
        assert_eq!(
            task_repository.list_dead_lettered().unwrap(),
            Vec::<DeadLetteredTask>::new()
        );
    }

    #[test]
    fn it_should_dead_letter_a_recovered_running_task_once_the_attempt_limit_is_exceeded() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(FakeHandler::succeeding());
        task_repository.schedule(&task(), now()).unwrap();
        task_repository
            .update(&pending_task(4).start(now()))
            .unwrap();
        let executor = TaskExecutor::new(
            task_repository.clone(),
            registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = executor.recover_stuck_tasks();

        assert!(result.is_ok());
        assert_eq!(
            *handler.received_is_last_attempt.lock().unwrap(),
            Vec::<bool>::new()
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            Vec::<ScheduledTask>::new()
        );
        assert_eq!(
            task_repository.list_dead_lettered().unwrap(),
            vec![dead_lettered_task(
                "recovered as a failed attempt after an unclean shutdown"
            )]
        );
    }

    const TEST_BASE_RETRY_DELAY_SECONDS: i64 = 150;

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn task() -> Task {
        Task::ReconcilePlaylist {
            playlist_id: "PL1".to_string(),
        }
    }

    /// The row `schedule(&task(), now())` stores as id 1, with `retries`.
    fn pending_task(retries: i64) -> ScheduledTask {
        ScheduledTask {
            id: 1,
            task_type: task().task_type().to_string(),
            payload: task().payload().to_string(),
            status: TaskStatus::Pending,
            retries,
            run_at: now(),
            created_at: now(),
            updated_at: now(),
            last_error: None,
        }
    }

    fn dead_lettered_task(last_error: &str) -> DeadLetteredTask {
        DeadLetteredTask {
            original_task_id: 1,
            task_type: task().task_type().to_string(),
            payload: task().payload().to_string(),
            retries: 5,
            last_error: last_error.to_string(),
            created_at: now(),
            failed_at: now(),
        }
    }

    fn registry(handler: Arc<FakeHandler>) -> HandlerRegistry {
        let mut registry: HandlerRegistry = HashMap::new();
        registry.insert(task().task_type().to_string(), handler);
        registry
    }

    struct FakeHandler {
        fails: bool,
        received_is_last_attempt: Mutex<Vec<bool>>,
    }

    impl FakeHandler {
        fn succeeding() -> Self {
            Self {
                fails: false,
                received_is_last_attempt: Mutex::new(Vec::new()),
            }
        }

        fn failing() -> Self {
            Self {
                fails: true,
                received_is_last_attempt: Mutex::new(Vec::new()),
            }
        }
    }

    impl TaskHandler for FakeHandler {
        fn handle(&self, _payload: &str, is_last_attempt: bool) -> anyhow::Result<()> {
            self.received_is_last_attempt
                .lock()
                .unwrap()
                .push(is_last_attempt);
            if self.fails {
                anyhow::bail!("handler failed");
            }
            Ok(())
        }
    }
}
