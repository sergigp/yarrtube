use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub type HandlerRegistry = HashMap<String, Arc<dyn TaskHandler>>;

/// Task-agnostic: polls `TaskRepository` for eligible tasks and dispatches
/// each to the single handler registered for its type.
pub struct TaskExecutor {
    repository: Arc<dyn TaskRepository>,
    handlers: HandlerRegistry,
}

impl TaskExecutor {
    pub fn new(repository: Arc<dyn TaskRepository>, handlers: HandlerRegistry) -> Self {
        Self {
            repository,
            handlers,
        }
    }

    pub fn poll_once(&self) -> anyhow::Result<()> {
        for task in self.repository.list_eligible()? {
            self.repository.mark_running(task.id)?;
            println!("[tasks] dispatching task {} ({})", task.id, task.task_type);

            let outcome = match self.handlers.get(&task.task_type) {
                Some(handler) => handler.handle(&task.payload),
                None => Err(anyhow::anyhow!(
                    "no handler registered for task type '{}'",
                    task.task_type
                )),
            };

            match outcome {
                Ok(()) => self.repository.mark_done(task.id)?,
                Err(e) => self
                    .repository
                    .mark_failed_or_retry(task.id, &e.to_string())?,
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
                Ok(Err(e)) => eprintln!("[tasks] poll failed: {e}"),
                Err(e) => eprintln!("[tasks] poll task panicked: {e}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::Task;
    use crate::infrastructure::repositories::sqlite_task_repository::PersistedTask;
    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeTaskRepository {
        tasks: Mutex<Vec<PersistedTask>>,
        running: Mutex<Vec<i64>>,
        done: Mutex<Vec<i64>>,
        retried: Mutex<Vec<i64>>,
    }

    impl FakeTaskRepository {
        fn seeded(task_type: &str) -> Self {
            let repo = Self::default();
            repo.tasks.lock().unwrap().push(PersistedTask {
                id: 1,
                task_type: task_type.to_string(),
                payload: Task::SyncPlaylist {
                    playlist_id: "PL1".to_string(),
                }
                .payload()
                .to_string(),
            });
            repo
        }
    }

    impl TaskRepository for FakeTaskRepository {
        fn schedule(&self, _task: &Task, _run_at: DateTime<Utc>) -> anyhow::Result<()> {
            unimplemented!("not exercised by the executor")
        }

        fn list_eligible(&self) -> anyhow::Result<Vec<PersistedTask>> {
            Ok(self.tasks.lock().unwrap().clone())
        }

        fn mark_running(&self, id: i64) -> anyhow::Result<()> {
            self.running.lock().unwrap().push(id);
            Ok(())
        }

        fn mark_done(&self, id: i64) -> anyhow::Result<()> {
            self.done.lock().unwrap().push(id);
            Ok(())
        }

        fn mark_failed_or_retry(&self, id: i64, _error: &str) -> anyhow::Result<()> {
            self.retried.lock().unwrap().push(id);
            Ok(())
        }

        fn recover_running(&self) -> anyhow::Result<()> {
            unimplemented!("not exercised by the executor")
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

    #[test]
    fn it_should_dispatch_an_eligible_task_and_mark_it_done_on_success() {
        let repository = Arc::new(FakeTaskRepository::seeded("sync_playlist"));
        let executor =
            TaskExecutor::new(repository.clone(), registry(FakeHandler { fails: false }));

        executor.poll_once().unwrap();

        assert_eq!(*repository.running.lock().unwrap(), vec![1]);
        assert_eq!(*repository.done.lock().unwrap(), vec![1]);
    }

    #[test]
    fn it_should_retry_a_task_whose_handler_fails() {
        let repository = Arc::new(FakeTaskRepository::seeded("sync_playlist"));
        let executor = TaskExecutor::new(repository.clone(), registry(FakeHandler { fails: true }));

        executor.poll_once().unwrap();

        assert_eq!(*repository.retried.lock().unwrap(), vec![1]);
        assert!(repository.done.lock().unwrap().is_empty());
    }
}
