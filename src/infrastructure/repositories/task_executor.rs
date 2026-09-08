use crate::domain::task::TaskFailureOutcome;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::system_clock::Clock;
use crate::infrastructure::repositories::task_handler::TaskHandler;
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
}

impl TaskExecutor {
    pub fn new(
        repository: Arc<dyn TaskRepository>,
        handlers: HandlerRegistry,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repository,
            handlers,
            clock,
        }
    }

    pub fn poll_once(&self) -> anyhow::Result<()> {
        for id in self.repository.list_eligible()? {
            let Some(task) = self.repository.find(id)? else {
                warn!(task_id = id, "eligible task disappeared before dispatch");
                continue;
            };
            let task_type = task.task_type.clone();
            let payload = task.payload.clone();
            let running = task.start(self.clock.now());
            self.repository.update(&running)?;
            info!(task_id = id, task_type = %task_type, "dispatching task");

            let outcome = match self.handlers.get(&task_type) {
                Some(handler) => handler.handle(&payload),
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
                    match running.fail(error.clone(), self.clock.now()) {
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
    use crate::infrastructure::repositories::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeTaskRepository {
        tasks: Mutex<Vec<ScheduledTask>>,
        running: Mutex<Vec<ScheduledTask>>,
        updated: Mutex<Vec<ScheduledTask>>,
        deleted: Mutex<Vec<i64>>,
        dead_lettered: Mutex<Vec<DeadLetteredTask>>,
    }

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn scheduled_task(id: i64, task_type: &str, retries: i64) -> ScheduledTask {
        ScheduledTask {
            id,
            task_type: task_type.to_string(),
            payload: Task::SyncPlaylist {
                playlist_id: "PL1".to_string(),
            }
            .payload()
            .to_string(),
            status: TaskStatus::Pending,
            retries,
            run_at: now(),
            created_at: now(),
            updated_at: now(),
            last_error: None,
        }
    }

    impl FakeTaskRepository {
        fn seeded(task_type: &str) -> Self {
            Self::seeded_with_retries(task_type, 0)
        }

        fn seeded_with_retries(task_type: &str, retries: i64) -> Self {
            let repo = Self::default();
            repo.tasks
                .lock()
                .unwrap()
                .push(scheduled_task(1, task_type, retries));
            repo
        }
    }

    impl TaskRepository for FakeTaskRepository {
        fn schedule(&self, _task: &Task, _run_at: DateTime<Utc>) -> anyhow::Result<()> {
            unimplemented!("not exercised by the executor")
        }

        fn find(&self, id: i64) -> anyhow::Result<Option<ScheduledTask>> {
            Ok(self
                .tasks
                .lock()
                .unwrap()
                .iter()
                .find(|t| t.id == id)
                .cloned())
        }

        fn list_eligible(&self) -> anyhow::Result<Vec<i64>> {
            Ok(self.tasks.lock().unwrap().iter().map(|t| t.id).collect())
        }

        fn list_running(&self) -> anyhow::Result<Vec<ScheduledTask>> {
            Ok(self.running.lock().unwrap().clone())
        }

        fn update(&self, task: &ScheduledTask) -> anyhow::Result<()> {
            self.updated.lock().unwrap().push(task.clone());
            Ok(())
        }

        fn delete(&self, id: i64) -> anyhow::Result<()> {
            self.deleted.lock().unwrap().push(id);
            Ok(())
        }

        fn dead_letter(&self, task: &DeadLetteredTask) -> anyhow::Result<()> {
            self.dead_lettered.lock().unwrap().push(task.clone());
            Ok(())
        }
    }

    struct FakeHandler {
        fails: bool,
    }

    impl TaskHandler for FakeHandler {
        fn handle(&self, _payload: &str) -> anyhow::Result<()> {
            if self.fails {
                anyhow::bail!("handler failed");
            }
            Ok(())
        }
    }

    fn registry(handler: FakeHandler) -> HandlerRegistry {
        let mut registry: HandlerRegistry = HashMap::new();
        registry.insert("sync_playlist".to_string(), Arc::new(handler));
        registry
    }

    fn clock() -> Arc<dyn Clock> {
        Arc::new(FixedClock(now()))
    }

    #[test]
    fn it_should_dispatch_an_eligible_task_and_delete_it_on_success() {
        let repository = Arc::new(FakeTaskRepository::seeded("sync_playlist"));
        let executor = TaskExecutor::new(
            repository.clone(),
            registry(FakeHandler { fails: false }),
            clock(),
        );

        executor.poll_once().unwrap();

        let updated = repository.updated.lock().unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].status, TaskStatus::Running);
        assert_eq!(*repository.deleted.lock().unwrap(), vec![1]);
    }

    #[test]
    fn it_should_retry_a_task_whose_handler_fails() {
        let repository = Arc::new(FakeTaskRepository::seeded("sync_playlist"));
        let executor = TaskExecutor::new(
            repository.clone(),
            registry(FakeHandler { fails: true }),
            clock(),
        );

        executor.poll_once().unwrap();

        let updated = repository.updated.lock().unwrap();
        assert_eq!(updated.len(), 2);
        assert_eq!(updated[1].status, TaskStatus::Pending);
        assert_eq!(updated[1].retries, 1);
        assert!(repository.deleted.lock().unwrap().is_empty());
        assert!(repository.dead_lettered.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_dead_letter_a_task_whose_handler_fails_on_the_fifth_attempt() {
        let repository = Arc::new(FakeTaskRepository::seeded_with_retries("sync_playlist", 4));
        let executor = TaskExecutor::new(
            repository.clone(),
            registry(FakeHandler { fails: true }),
            clock(),
        );

        executor.poll_once().unwrap();

        let dead_lettered = repository.dead_lettered.lock().unwrap();
        assert_eq!(dead_lettered.len(), 1);
        assert_eq!(dead_lettered[0].retries, 5);
        assert!(repository.deleted.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_retry_a_task_recovered_as_running_from_a_previous_process() {
        let repository = Arc::new(FakeTaskRepository::default());
        repository
            .running
            .lock()
            .unwrap()
            .push(scheduled_task(1, "sync_playlist", 0));
        let executor = TaskExecutor::new(
            repository.clone(),
            registry(FakeHandler { fails: false }),
            clock(),
        );

        executor.recover_stuck_tasks().unwrap();

        let updated = repository.updated.lock().unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].status, TaskStatus::Pending);
        assert_eq!(updated[0].retries, 1);
    }

    #[test]
    fn it_should_dead_letter_a_recovered_running_task_once_the_attempt_limit_is_exceeded() {
        let repository = Arc::new(FakeTaskRepository::default());
        repository
            .running
            .lock()
            .unwrap()
            .push(scheduled_task(1, "sync_playlist", 4));
        let executor = TaskExecutor::new(
            repository.clone(),
            registry(FakeHandler { fails: false }),
            clock(),
        );

        executor.recover_stuck_tasks().unwrap();

        let dead_lettered = repository.dead_lettered.lock().unwrap();
        assert_eq!(dead_lettered.len(), 1);
        assert_eq!(dead_lettered[0].retries, 5);
        assert!(repository.updated.lock().unwrap().is_empty());
    }
}
