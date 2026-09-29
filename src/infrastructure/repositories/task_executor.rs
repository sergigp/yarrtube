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
}

/// What a running task holds while it runs: a slot in its lane, and its
/// exclusivity key, if it has one.
struct Slot {
    lane: TaskLane,
    key: Option<String>,
}

impl Slot {
    fn for_task(task: &ScheduledTask) -> Self {
        Self {
            lane: Task::lane_for(&task.task_type),
            key: Task::exclusivity_key(&task.task_type, &task.payload),
        }
    }
}

impl SchedulerState {
    fn running_in(&self, lane: TaskLane) -> usize {
        self.running.get(&lane).copied().unwrap_or(0)
    }

    fn exclusive_running(&self) -> bool {
        self.running_in(TaskLane::Exclusive) > 0
    }

    fn is_idle(&self) -> bool {
        self.running.values().all(|count| *count == 0)
    }

    fn is_held(&self, key: Option<&String>) -> bool {
        key.is_some_and(|key| self.held_keys.contains(key))
    }

    fn start(&mut self, slot: &Slot) {
        *self.running.entry(slot.lane).or_insert(0) += 1;
        if let Some(key) = &slot.key {
            self.held_keys.insert(key.clone());
        }
    }

    fn finish(&mut self, slot: &Slot) {
        if let Some(count) = self.running.get_mut(&slot.lane) {
            *count = count.saturating_sub(1);
        }
        if let Some(key) = &slot.key {
            self.held_keys.remove(key);
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
        if state.exclusive_running() {
            return Ok(Vec::new());
        }
        let eligible = self.repository.list_eligible()?;
        match eligible
            .iter()
            .find(|task| Task::lane_for(&task.task_type) == TaskLane::Exclusive)
        {
            Some(exclusive) => self.start_exclusive(&mut state, exclusive),
            None => self.start_fitting(&mut state, &eligible),
        }
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

    /// An exclusive task starts only once nothing else runs; until then no
    /// task starts at all, so the running ones drain.
    fn start_exclusive(
        self: &Arc<Self>,
        state: &mut SchedulerState,
        exclusive: &ScheduledTask,
    ) -> anyhow::Result<Vec<JoinHandle<()>>> {
        if !state.is_idle() {
            return Ok(Vec::new());
        }
        Ok(self.start(state, exclusive)?.into_iter().collect())
    }

    /// Starts, in eligibility order, every task whose lane has room and whose
    /// exclusivity key is free; the rest are passed over, not waited for.
    fn start_fitting(
        self: &Arc<Self>,
        state: &mut SchedulerState,
        eligible: &[ScheduledTask],
    ) -> anyhow::Result<Vec<JoinHandle<()>>> {
        let mut handles = Vec::new();
        for task in eligible {
            let slot = Slot::for_task(task);
            if state.running_in(slot.lane) >= self.capacity(slot.lane)
                || state.is_held(slot.key.as_ref())
            {
                continue;
            }
            handles.extend(self.start(state, task)?);
        }
        Ok(handles)
    }

    /// Claims `task` and spawns it; `None` if it was claimed elsewhere first.
    fn start(
        self: &Arc<Self>,
        state: &mut SchedulerState,
        task: &ScheduledTask,
    ) -> anyhow::Result<Option<JoinHandle<()>>> {
        let Some(running) = self.repository.claim(task.id, self.clock.now())? else {
            return Ok(None);
        };
        let slot = Slot::for_task(&running);
        state.start(&slot);
        Ok(Some(self.spawn(running, slot)))
    }

    fn capacity(&self, lane: TaskLane) -> usize {
        match lane {
            TaskLane::Download => self.download_concurrency,
            _ => 1,
        }
    }

    /// Runs `running` on the blocking pool, then frees its slot (lane and
    /// key) and wakes the scheduler so it is refilled straight away.
    fn spawn(self: &Arc<Self>, running: ScheduledTask, slot: Slot) -> JoinHandle<()> {
        let executor = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            let id = running.id;
            if let Err(e) = executor.execute(running) {
                error!(task_id = id, error = %e, "failed to record task outcome");
            }
            match executor.lock_state() {
                Ok(mut state) => state.finish(&slot),
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

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_dispatch_an_eligible_task_and_delete_it_on_success() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(FakeHandler::succeeding());
        task_repository.schedule(&task(), now()).unwrap();
        let executor = Arc::new(TaskExecutor::new(
            task_repository.clone(),
            registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
            TEST_DOWNLOAD_CONCURRENCY,
        ));

        let result = run_pass(&executor).await;

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

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_retry_a_task_whose_handler_fails() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(FakeHandler::failing());
        task_repository.schedule(&task(), now()).unwrap();
        let executor = Arc::new(TaskExecutor::new(
            task_repository.clone(),
            registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
            TEST_DOWNLOAD_CONCURRENCY,
        ));

        let result = run_pass(&executor).await;

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

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_dead_letter_a_task_whose_handler_fails_on_the_fifth_attempt() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(FakeHandler::failing());
        task_repository.schedule(&task(), now()).unwrap();
        task_repository.update(&pending_task(4)).unwrap();
        let executor = Arc::new(TaskExecutor::new(
            task_repository.clone(),
            registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
            TEST_DOWNLOAD_CONCURRENCY,
        ));

        let result = run_pass(&executor).await;

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

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_tell_the_handler_this_is_not_the_last_attempt_when_retries_remain() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(FakeHandler::succeeding());
        task_repository.schedule(&task(), now()).unwrap();
        task_repository.update(&pending_task(3)).unwrap();
        let executor = Arc::new(TaskExecutor::new(
            task_repository.clone(),
            registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
            TEST_DOWNLOAD_CONCURRENCY,
        ));

        let result = run_pass(&executor).await;

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

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_tell_the_handler_this_is_the_last_attempt_when_no_retries_remain() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(FakeHandler::succeeding());
        task_repository.schedule(&task(), now()).unwrap();
        task_repository.update(&pending_task(4)).unwrap();
        let executor = Arc::new(TaskExecutor::new(
            task_repository.clone(),
            registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
            TEST_DOWNLOAD_CONCURRENCY,
        ));

        let result = run_pass(&executor).await;

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

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_not_block_light_tasks_behind_downloads() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(BlockingHandler::default());
        let _releases = [
            handler.gate(&download("rec1")),
            handler.gate(&reconcile("PL1")),
        ];
        task_repository.schedule(&download("rec1"), now()).unwrap();
        task_repository.schedule(&reconcile("PL1"), now()).unwrap();
        let executor = Arc::new(TaskExecutor::new(
            task_repository.clone(),
            blocking_registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
            1,
        ));

        executor.schedule_pass().unwrap();

        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![running(1, &download("rec1")), running(2, &reconcile("PL1")),]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_run_light_tasks_one_at_a_time() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(BlockingHandler::default());
        let releases = [
            handler.gate(&reconcile("PL1")),
            handler.gate(&reconcile("PL2")),
        ];
        task_repository.schedule(&reconcile("PL1"), now()).unwrap();
        task_repository.schedule(&reconcile("PL2"), now()).unwrap();
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
            vec![running(1, &reconcile("PL1")), pending(2, &reconcile("PL2")),]
        );

        release(&releases[0], handles.remove(0)).await;
        executor.schedule_pass().unwrap();

        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![running(2, &reconcile("PL2"))]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_not_block_reconciles_behind_thumbnail_fetches() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(BlockingHandler::default());
        let _releases = [
            handler.gate(&fetch_thumbnail("rec1")),
            handler.gate(&reconcile("PL1")),
        ];
        task_repository
            .schedule(&fetch_thumbnail("rec1"), now())
            .unwrap();
        task_repository
            .schedule(&fetch_thumbnail("rec2"), now())
            .unwrap();
        task_repository.schedule(&reconcile("PL1"), now()).unwrap();
        let executor = Arc::new(TaskExecutor::new(
            task_repository.clone(),
            blocking_registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
            2,
        ));

        executor.schedule_pass().unwrap();

        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                running(1, &fetch_thumbnail("rec1")),
                pending(2, &fetch_thumbnail("rec2")),
                running(3, &reconcile("PL1")),
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_run_thumbnail_fetches_one_at_a_time() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(BlockingHandler::default());
        let releases = [
            handler.gate(&fetch_thumbnail("rec1")),
            handler.gate(&fetch_thumbnail("rec2")),
        ];
        task_repository
            .schedule(&fetch_thumbnail("rec1"), now())
            .unwrap();
        task_repository
            .schedule(&fetch_thumbnail("rec2"), now())
            .unwrap();
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
                running(1, &fetch_thumbnail("rec1")),
                pending(2, &fetch_thumbnail("rec2")),
            ]
        );

        release(&releases[0], handles.remove(0)).await;
        executor.schedule_pass().unwrap();

        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![running(2, &fetch_thumbnail("rec2"))]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_not_run_two_tasks_for_the_same_video_at_once() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(BlockingHandler::default());
        let releases = [
            handler.gate(&download("rec1")),
            handler.gate(&fetch_thumbnail("rec1")),
        ];
        task_repository.schedule(&download("rec1"), now()).unwrap();
        task_repository
            .schedule(&fetch_thumbnail("rec1"), now())
            .unwrap();
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
                pending(2, &fetch_thumbnail("rec1")),
            ]
        );

        release(&releases[0], handles.remove(0)).await;
        executor.schedule_pass().unwrap();

        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![running(2, &fetch_thumbnail("rec1"))]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_start_a_later_task_when_the_earlier_one_is_held_back() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(BlockingHandler::default());
        let _releases = [
            handler.gate(&download("rec1")),
            handler.gate(&fetch_thumbnail("rec2")),
        ];
        task_repository.schedule(&download("rec1"), now()).unwrap();
        task_repository
            .schedule(&fetch_thumbnail("rec1"), now())
            .unwrap();
        task_repository
            .schedule(&fetch_thumbnail("rec2"), now())
            .unwrap();
        let executor = Arc::new(TaskExecutor::new(
            task_repository.clone(),
            blocking_registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
            2,
        ));

        executor.schedule_pass().unwrap();

        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                running(1, &download("rec1")),
                pending(2, &fetch_thumbnail("rec1")),
                running(3, &fetch_thumbnail("rec2")),
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_run_update_ytdlp_only_when_nothing_else_runs() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(BlockingHandler::default());
        let releases = [
            handler.gate(&download("rec1")),
            handler.gate(&Task::UpdateYtdlp),
        ];
        task_repository.schedule(&download("rec1"), now()).unwrap();
        let executor = Arc::new(TaskExecutor::new(
            task_repository.clone(),
            blocking_registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
            2,
        ));
        let mut handles = executor.schedule_pass().unwrap();
        task_repository.schedule(&Task::UpdateYtdlp, now()).unwrap();
        task_repository.schedule(&download("rec2"), now()).unwrap();

        executor.schedule_pass().unwrap();

        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                running(1, &download("rec1")),
                pending(2, &Task::UpdateYtdlp),
                pending(3, &download("rec2")),
            ]
        );

        release(&releases[0], handles.remove(0)).await;
        executor.schedule_pass().unwrap();

        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                running(2, &Task::UpdateYtdlp),
                pending(3, &download("rec2")),
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_not_start_tasks_while_update_ytdlp_runs() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(BlockingHandler::default());
        let releases = [
            handler.gate(&Task::UpdateYtdlp),
            handler.gate(&reconcile("PL1")),
        ];
        task_repository.schedule(&Task::UpdateYtdlp, now()).unwrap();
        let executor = Arc::new(TaskExecutor::new(
            task_repository.clone(),
            blocking_registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
            2,
        ));
        let mut handles = executor.schedule_pass().unwrap();
        task_repository.schedule(&reconcile("PL1"), now()).unwrap();

        executor.schedule_pass().unwrap();

        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                running(1, &Task::UpdateYtdlp),
                pending(2, &reconcile("PL1")),
            ]
        );

        release(&releases[0], handles.remove(0)).await;
        executor.schedule_pass().unwrap();

        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![running(2, &reconcile("PL1"))]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn it_should_start_tasks_of_a_lane_in_run_at_order() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(now())),
        ));
        let handler = Arc::new(BlockingHandler::default());
        let _releases = [
            handler.gate(&download("rec1")),
            handler.gate(&download("rec2")),
        ];
        let earlier = now() - chrono::Duration::seconds(60);
        task_repository.schedule(&download("rec1"), now()).unwrap();
        task_repository
            .schedule(&download("rec2"), earlier)
            .unwrap();
        let executor = Arc::new(TaskExecutor::new(
            task_repository.clone(),
            blocking_registry(handler.clone()),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
            1,
        ));

        executor.schedule_pass().unwrap();

        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending(1, &download("rec1")),
                ScheduledTask {
                    run_at: earlier,
                    ..running(2, &download("rec2"))
                },
            ]
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

    fn fetch_thumbnail(video_id: &str) -> Task {
        Task::FetchThumbnail {
            video_id: video_id.to_string(),
            output_dir: "/videos/my-playlist".to_string(),
        }
    }

    fn reconcile(playlist_id: &str) -> Task {
        Task::ReconcilePlaylist {
            playlist_id: playlist_id.to_string(),
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

    /// Runs one scheduling pass and waits for every task it started.
    async fn run_pass(executor: &Arc<TaskExecutor>) -> anyhow::Result<()> {
        for handle in executor.schedule_pass()? {
            handle.await?;
        }
        Ok(())
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
