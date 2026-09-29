use crate::domain::task::{ScheduledTask, Task, TaskFailureOutcome, TaskLane};
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

pub type HandlerRegistry = HashMap<String, Arc<dyn TaskHandler>>;

/// Task-agnostic: polls `TaskRepository` for eligible tasks and dispatches
/// each to the single handler registered for its type.
pub struct TaskExecutor {
    repository: Arc<dyn TaskRepository>,
    handlers: HandlerRegistry,
    clock: Arc<dyn Clock>,
    base_retry_delay_seconds: i64,
    download_concurrency: usize,
    state: Mutex<SchedulerState>,
    wake: Notify,
}

/// What is running right now, in memory: there is exactly one executor per
/// process, and `recover_stuck_tasks` requeues every `running` row at
/// startup, so an empty state after a restart is correct.
#[derive(Default)]
struct SchedulerState {
    running: HashMap<TaskLane, usize>,
    held_keys: HashSet<String>,
    exclusive_running: bool,
}

impl SchedulerState {
    fn running_in(&self, lane: TaskLane) -> usize {
        self.running.get(&lane).copied().unwrap_or(0)
    }

    fn start(&mut self, lane: TaskLane) {
        *self.running.entry(lane).or_insert(0) += 1;
    }

    fn finish(&mut self, lane: TaskLane) {
        if let Some(count) = self.running.get_mut(&lane) {
            *count = count.saturating_sub(1);
        }
    }
}

impl TaskExecutor {
    pub fn new(
        repository: Arc<dyn TaskRepository>,
        handlers: HandlerRegistry,
        clock: Arc<dyn Clock>,
        base_retry_delay_seconds: i64,
        download_concurrency: usize,
    ) -> Self {
        Self {
            repository,
            handlers,
            clock,
            base_retry_delay_seconds,
            download_concurrency,
            state: Mutex::new(SchedulerState::default()),
            wake: Notify::new(),
        }
    }

    /// One scheduling pass: claims and spawns every eligible task that fits
    /// (lane capacity, held keys, exclusive drain). Returns the handles of
    /// the tasks it started, so tests can await them.
    pub fn schedule_pass(self: &Arc<Self>) -> anyhow::Result<Vec<JoinHandle<()>>> {
        let mut state = self.lock_state()?;
        let mut handles = Vec::new();
        for task in self.repository.list_eligible()? {
            let lane = Task::lane_for(&task.task_type);
            if state.running_in(lane) >= self.capacity(lane) {
                continue;
            }
            let Some(running) = self.repository.claim(task.id, self.clock.now())? else {
                continue;
            };
            state.start(lane);
            handles.push(self.spawn(running, lane));
        }
        Ok(handles)
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
            match tokio::task::spawn_blocking(move || executor.schedule_pass()).await {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => error!(error = %e, "poll failed"),
                Err(e) => error!(error = %e, "poll task panicked"),
            }
        }
    }

    fn lock_state(&self) -> anyhow::Result<MutexGuard<'_, SchedulerState>> {
        self.state
            .lock()
            .map_err(|_| anyhow::anyhow!("task scheduler state lock poisoned"))
    }

    fn capacity(&self, _lane: TaskLane) -> usize {
        self.download_concurrency
    }

    /// Runs `running` on the blocking pool, then frees its lane slot and
    /// wakes the scheduler so the slot is refilled straight away.
    fn spawn(self: &Arc<Self>, running: ScheduledTask, lane: TaskLane) -> JoinHandle<()> {
        let executor = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            let id = running.id;
            if let Err(e) = executor.execute(running) {
                error!(task_id = id, error = %e, "failed to record task outcome");
            }
            match executor.lock_state() {
                Ok(mut state) => state.finish(lane),
                Err(e) => error!(task_id = id, error = %e, "failed to release task slot"),
            }
            executor.wake.notify_one();
        })
    }

    /// Dispatches a claimed task and records its outcome: deleted on
    /// success, retried or dead-lettered on failure.
    fn execute(&self, running: ScheduledTask) -> anyhow::Result<()> {
        let id = running.id;
        info!(task_id = id, task_type = %running.task_type, "dispatching task");
        match self.dispatch(&running) {
            Ok(()) => {
                self.repository.delete(id)?;
                info!(task_id = id, "task done");
                Ok(())
            }
            Err(e) => self.record_failure(running, e),
        }
    }

    fn dispatch(&self, running: &ScheduledTask) -> anyhow::Result<()> {
        match self.handlers.get(&running.task_type) {
            Some(handler) => handler.handle(&running.payload, running.is_last_attempt()),
            None => Err(anyhow::anyhow!(
                "no handler registered for task type '{}'",
                running.task_type
            )),
        }
    }

    fn record_failure(&self, running: ScheduledTask, error: anyhow::Error) -> anyhow::Result<()> {
        let id = running.id;
        let error = error.to_string();
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
                self.repository.update(&retried)
            }
            TaskFailureOutcome::DeadLetter(dead) => {
                error!(
                    task_id = id,
                    retries = dead.retries,
                    error,
                    "task failed permanently"
                );
                self.repository.dead_letter(&dead)
            }
        }
    }

    /// Runs every eligible task inline, one at a time. Only the tests not
    /// yet moved to `schedule_pass` still use it.
    #[cfg(test)]
    fn poll_once(&self) -> anyhow::Result<()> {
        for task in self.repository.list_eligible()? {
            let running = task.start(self.clock.now());
            self.repository.update(&running)?;
            self.execute(running)?;
        }
        Ok(())
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
    use std::sync::mpsc::{Receiver, Sender, channel};

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
            TEST_DOWNLOAD_CONCURRENCY,
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
            TEST_DOWNLOAD_CONCURRENCY,
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
            TEST_DOWNLOAD_CONCURRENCY,
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
            TEST_DOWNLOAD_CONCURRENCY,
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
            TEST_DOWNLOAD_CONCURRENCY,
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
            TEST_DOWNLOAD_CONCURRENCY,
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
            TEST_DOWNLOAD_CONCURRENCY,
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

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_run_downloads_in_parallel_up_to_the_lane_concurrency() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(BlockingHandler::default());
        let releases = [
            handler.gate(&download("rec1")),
            handler.gate(&download("rec2")),
            handler.gate(&download("rec3")),
        ];
        [download("rec1"), download("rec2"), download("rec3")]
            .iter()
            .for_each(|task| task_repository.schedule(task, now()).unwrap());
        let executor = Arc::new(TaskExecutor::new(
            task_repository.clone(),
            blocking_registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
            2,
        ));

        let mut handles = executor.schedule_pass().unwrap();

        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                running(1, &download("rec1")),
                running(2, &download("rec2")),
                pending(3, &download("rec3")),
            ]
        );

        release(&releases[0], handles.remove(0)).await;
        executor.schedule_pass().unwrap();

        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![running(2, &download("rec2")), running(3, &download("rec3")),]
        );
    }

    const TEST_BASE_RETRY_DELAY_SECONDS: i64 = 150;
    const TEST_DOWNLOAD_CONCURRENCY: usize = 2;

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

    fn download(video_id: &str) -> Task {
        Task::DownloadVideo {
            video_id: video_id.to_string(),
            quality: "high".to_string(),
            output_dir: "/videos/my-playlist".to_string(),
        }
    }

    fn pending(id: i64, task: &Task) -> ScheduledTask {
        ScheduledTask {
            id,
            task_type: task.task_type().to_string(),
            payload: task.payload().to_string(),
            status: TaskStatus::Pending,
            retries: 0,
            run_at: now(),
            created_at: now(),
            updated_at: now(),
            last_error: None,
        }
    }

    fn running(id: i64, task: &Task) -> ScheduledTask {
        pending(id, task).start(now())
    }

    /// Opens `gate`, letting its blocked task finish, and waits for the
    /// executor's bookkeeping of that task to complete.
    async fn release(gate: &Sender<()>, handle: JoinHandle<()>) {
        gate.send(()).unwrap();
        handle.await.unwrap();
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

    /// Every task type handled by one `BlockingHandler`.
    fn blocking_registry(handler: Arc<BlockingHandler>) -> HandlerRegistry {
        [
            "download_video",
            "fetch_thumbnail",
            "reconcile_playlist",
            "update_ytdlp",
        ]
        .into_iter()
        .map(|task_type| {
            (
                task_type.to_string(),
                handler.clone() as Arc<dyn TaskHandler>,
            )
        })
        .collect()
    }

    /// Records every payload it handles, and holds a task whose payload has
    /// a gate until that gate is opened (or dropped).
    #[derive(Default)]
    struct BlockingHandler {
        started: Mutex<Vec<String>>,
        gates: Mutex<HashMap<String, Receiver<()>>>,
    }

    impl BlockingHandler {
        fn gate(&self, task: &Task) -> Sender<()> {
            let (sender, receiver) = channel();
            self.gates
                .lock()
                .unwrap()
                .insert(task.payload().to_string(), receiver);
            sender
        }
    }

    impl TaskHandler for BlockingHandler {
        fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
            self.started.lock().unwrap().push(payload.to_string());
            let gate = self.gates.lock().unwrap().remove(payload);
            if let Some(gate) = gate {
                let _ = gate.recv();
            }
            Ok(())
        }
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
