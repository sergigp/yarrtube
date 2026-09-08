use crate::domain::task::{DeadLetteredTask, ScheduledTask, Task, TaskStatus};
use crate::infrastructure::repositories::system_clock::Clock;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params};
use std::sync::{Arc, Mutex};
use tracing::info;

pub trait TaskRepository: Send + Sync {
    fn schedule(&self, task: &Task, run_at: DateTime<Utc>) -> anyhow::Result<()>;
    fn find(&self, id: i64) -> anyhow::Result<Option<ScheduledTask>>;
    /// IDs of tasks ready to run now, cheapest read for the hot poll loop —
    /// callers `find` each one right before mutating it.
    fn list_eligible(&self) -> anyhow::Result<Vec<i64>>;
    /// Every task still `running` from a previous, interrupted process.
    fn list_running(&self) -> anyhow::Result<Vec<ScheduledTask>>;
    fn update(&self, task: &ScheduledTask) -> anyhow::Result<()>;
    fn delete(&self, id: i64) -> anyhow::Result<()>;
    /// Atomically inserts `task` into the dead-letter table and deletes the
    /// original row.
    fn dead_letter(&self, task: &DeadLetteredTask) -> anyhow::Result<()>;
}

fn row_to_scheduled_task(row: &Row) -> rusqlite::Result<ScheduledTask> {
    let status: String = row.get(3)?;
    let run_at: String = row.get(5)?;
    let created_at: String = row.get(6)?;
    let updated_at: String = row.get(7)?;
    Ok(ScheduledTask {
        id: row.get(0)?,
        task_type: row.get(1)?,
        payload: row.get(2)?,
        status: TaskStatus::parse(&status).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                3,
                rusqlite::types::Type::Text,
                e.to_string().into(),
            )
        })?,
        retries: row.get(4)?,
        run_at: DateTime::parse_from_rfc3339(&run_at)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
            .with_timezone(&Utc),
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
            .with_timezone(&Utc),
        last_error: row.get(8)?,
    })
}

const SELECT_COLUMNS: &str =
    "id, task_type, payload, status, retries, run_at, created_at, updated_at, last_error";

pub struct SqliteTaskRepository {
    conn: Arc<Mutex<Connection>>,
    clock: Arc<dyn Clock>,
}

impl SqliteTaskRepository {
    pub fn new(conn: Arc<Mutex<Connection>>, clock: Arc<dyn Clock>) -> anyhow::Result<Self> {
        {
            let guard = conn
                .lock()
                .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
            guard
                .execute(
                    "CREATE TABLE IF NOT EXISTS tasks (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        task_type TEXT NOT NULL,
                        payload TEXT NOT NULL,
                        status TEXT NOT NULL,
                        retries INTEGER NOT NULL DEFAULT 0,
                        run_at TEXT NOT NULL,
                        created_at TEXT NOT NULL,
                        updated_at TEXT NOT NULL,
                        last_error TEXT
                    )",
                    [],
                )
                .context("failed to create tasks table")?;
            guard
                .execute(
                    "CREATE TABLE IF NOT EXISTS tasks_dead_letter (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        original_task_id INTEGER NOT NULL,
                        task_type TEXT NOT NULL,
                        payload TEXT NOT NULL,
                        retries INTEGER NOT NULL,
                        last_error TEXT,
                        created_at TEXT NOT NULL,
                        failed_at TEXT NOT NULL
                    )",
                    [],
                )
                .context("failed to create tasks_dead_letter table")?;
        }
        Ok(Self { conn, clock })
    }

    fn list_where(&self, predicate: &str) -> anyhow::Result<Vec<ScheduledTask>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let query = format!("SELECT {SELECT_COLUMNS} FROM tasks WHERE {predicate} ORDER BY id ASC");
        let mut stmt = conn
            .prepare(&query)
            .context("failed to prepare tasks query")?;
        let rows = stmt
            .query_map([], row_to_scheduled_task)
            .context("failed to list tasks")?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("failed to read task row")
    }
}

impl TaskRepository for SqliteTaskRepository {
    fn schedule(&self, task: &Task, run_at: DateTime<Utc>) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let now = self.clock.now().to_rfc3339();
        conn.execute(
            "INSERT INTO tasks (task_type, payload, status, retries, run_at, created_at, updated_at, last_error)
             VALUES (?1, ?2, 'pending', 0, ?3, ?4, ?4, NULL)",
            params![
                task.task_type(),
                task.payload().to_string(),
                run_at.to_rfc3339(),
                now
            ],
        )
        .context("failed to schedule task")?;
        info!(
            task_id = conn.last_insert_rowid(),
            task_type = task.task_type(),
            run_at = %run_at,
            "scheduled task"
        );
        Ok(())
    }

    fn find(&self, id: i64) -> anyhow::Result<Option<ScheduledTask>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.query_row(
            &format!("SELECT {SELECT_COLUMNS} FROM tasks WHERE id = ?1"),
            params![id],
            row_to_scheduled_task,
        )
        .optional()
        .context("failed to find task")
    }

    fn list_eligible(&self) -> anyhow::Result<Vec<i64>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut stmt = conn
            .prepare(
                "SELECT id FROM tasks WHERE status = 'pending' AND run_at <= ?1 ORDER BY id ASC",
            )
            .context("failed to prepare list-eligible-tasks query")?;
        let rows = stmt
            .query_map(params![self.clock.now().to_rfc3339()], |row| row.get(0))
            .context("failed to list eligible tasks")?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("failed to read eligible task id")
    }

    fn list_running(&self) -> anyhow::Result<Vec<ScheduledTask>> {
        self.list_where("status = 'running'")
    }

    fn update(&self, task: &ScheduledTask) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "UPDATE tasks SET status = ?2, retries = ?3, run_at = ?4, updated_at = ?5, last_error = ?6 WHERE id = ?1",
            params![
                task.id,
                task.status.as_str(),
                task.retries,
                task.run_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.last_error,
            ],
        )
        .context("failed to update task")?;
        Ok(())
    }

    fn delete(&self, id: i64) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])
            .context("failed to delete task")?;
        Ok(())
    }

    fn dead_letter(&self, task: &DeadLetteredTask) -> anyhow::Result<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let tx = conn
            .transaction()
            .context("failed to start dead-letter transaction")?;
        tx.execute(
            "INSERT INTO tasks_dead_letter (original_task_id, task_type, payload, retries, last_error, created_at, failed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                task.original_task_id,
                task.task_type,
                task.payload,
                task.retries,
                task.last_error,
                task.created_at.to_rfc3339(),
                task.failed_at.to_rfc3339(),
            ],
        )
        .context("failed to insert task into dead letter")?;
        tx.execute(
            "DELETE FROM tasks WHERE id = ?1",
            params![task.original_task_id],
        )
        .context("failed to delete task after moving to dead letter")?;
        tx.commit()
            .context("failed to commit dead-letter transaction")?;
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeTaskRepository {
    pub(crate) scheduled: Mutex<Vec<(Task, DateTime<Utc>)>>,
}

#[cfg(test)]
impl TaskRepository for FakeTaskRepository {
    fn schedule(&self, task: &Task, run_at: DateTime<Utc>) -> anyhow::Result<()> {
        self.scheduled.lock().unwrap().push((task.clone(), run_at));
        Ok(())
    }

    fn find(&self, _id: i64) -> anyhow::Result<Option<ScheduledTask>> {
        Ok(None)
    }

    fn list_eligible(&self) -> anyhow::Result<Vec<i64>> {
        Ok(Vec::new())
    }

    fn list_running(&self) -> anyhow::Result<Vec<ScheduledTask>> {
        Ok(Vec::new())
    }

    fn update(&self, _task: &ScheduledTask) -> anyhow::Result<()> {
        Ok(())
    }

    fn delete(&self, _id: i64) -> anyhow::Result<()> {
        Ok(())
    }

    fn dead_letter(&self, _task: &DeadLetteredTask) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::TaskFailureOutcome;
    use crate::infrastructure::repositories::system_clock::FixedClock;

    fn repo_with_clock(now: DateTime<Utc>) -> SqliteTaskRepository {
        SqliteTaskRepository::new(
            Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
            Arc::new(FixedClock(now)),
        )
        .unwrap()
    }

    fn repo() -> SqliteTaskRepository {
        repo_with_clock(DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap())
    }

    fn task() -> Task {
        Task::SyncPlaylist {
            playlist_id: "PL1".to_string(),
        }
    }

    /// Mirrors what `TaskExecutor` does on a dispatch failure: read the task,
    /// let it decide retry vs. dead-letter, then persist that decision.
    fn apply_failure(repo: &SqliteTaskRepository, id: i64, error: &str) {
        let scheduled = repo.find(id).unwrap().unwrap();
        match scheduled.fail(error, repo.clock.now()) {
            TaskFailureOutcome::Retry(retried) => repo.update(&retried).unwrap(),
            TaskFailureOutcome::DeadLetter(dead) => repo.dead_letter(&dead).unwrap(),
        }
    }

    #[test]
    fn it_should_list_a_task_scheduled_for_now_as_eligible() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);

        repo.schedule(&task(), now).unwrap();

        let eligible = repo.list_eligible().unwrap();
        assert_eq!(eligible.len(), 1);
        assert_eq!(
            repo.find(eligible[0]).unwrap().unwrap().task_type,
            "sync_playlist"
        );
    }

    #[test]
    fn it_should_not_list_a_task_scheduled_for_the_future_as_eligible() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        let future = now + chrono::Duration::seconds(60);

        repo.schedule(&task(), future).unwrap();

        assert!(repo.list_eligible().unwrap().is_empty());
    }

    #[test]
    fn it_should_no_longer_list_a_task_as_eligible_once_deleted() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0];

        repo.delete(id).unwrap();

        assert!(repo.list_eligible().unwrap().is_empty());
        assert!(repo.find(id).unwrap().is_none());
    }

    #[test]
    fn it_should_create_a_queryable_empty_dead_letter_table() {
        let repo = repo();

        let conn = repo.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM tasks_dead_letter").unwrap();
        let rows = stmt.query_map([], |_| Ok(())).unwrap();

        assert_eq!(rows.count(), 0);
    }

    #[test]
    fn it_should_no_longer_list_a_running_task_as_eligible() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0];
        let scheduled = repo.find(id).unwrap().unwrap();

        repo.update(&scheduled.start(repo.clock.now())).unwrap();

        assert!(repo.list_eligible().unwrap().is_empty());
    }

    #[test]
    fn it_should_drop_the_task_after_the_fifth_failed_attempt() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0];

        for _ in 0..5 {
            apply_failure(&repo, id, "boom");
        }

        assert!(repo.list_eligible().unwrap().is_empty());
        assert!(repo.find(id).unwrap().is_none());

        let conn = repo.conn.lock().unwrap();
        let (original_task_id, task_type, payload, retries, last_error): (
            i64,
            String,
            String,
            i64,
            String,
        ) = conn
            .query_row(
                "SELECT original_task_id, task_type, payload, retries, last_error FROM tasks_dead_letter",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .unwrap();
        assert_eq!(original_task_id, id);
        assert_eq!(task_type, "sync_playlist");
        assert_eq!(payload, task().payload().to_string());
        assert_eq!(retries, 5);
        assert_eq!(last_error, "boom");
    }

    #[test]
    fn it_should_reschedule_the_task_after_the_fixed_delay_below_the_attempt_limit() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        repo.schedule(&task(), now).unwrap();
        let id = repo.list_eligible().unwrap()[0];

        apply_failure(&repo, id, "boom");

        assert!(repo.list_eligible().unwrap().is_empty());
        let found = repo.find(id).unwrap().unwrap();
        assert_eq!(found.run_at, now + chrono::Duration::seconds(30));
        assert_eq!(found.retries, 1);
        assert_eq!(found.last_error, Some("boom".to_string()));
    }

    #[test]
    fn it_should_retry_a_recovered_running_task_when_under_the_attempt_limit() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0];
        let scheduled = repo.find(id).unwrap().unwrap();
        repo.update(&scheduled.start(repo.clock.now())).unwrap();

        let running = repo.list_running().unwrap();
        assert_eq!(running.len(), 1);
        match running.into_iter().next().unwrap().fail(
            "recovered as a failed attempt after an unclean shutdown",
            repo.clock.now(),
        ) {
            TaskFailureOutcome::Retry(retried) => repo.update(&retried).unwrap(),
            TaskFailureOutcome::DeadLetter(_) => panic!("expected a retry outcome"),
        }

        let found = repo.find(id).unwrap().unwrap();
        assert_eq!(found.status, TaskStatus::Pending);
        assert_eq!(found.retries, 1);
        assert!(repo.list_running().unwrap().is_empty());
    }

    #[test]
    fn it_should_drop_a_recovered_running_task_once_the_attempt_limit_is_exceeded() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0];

        for _ in 0..4 {
            apply_failure(&repo, id, "boom");
        }
        let scheduled = repo.find(id).unwrap().unwrap();
        repo.update(&scheduled.start(repo.clock.now())).unwrap();

        let running = repo.list_running().unwrap();
        assert_eq!(running.len(), 1);
        match running.into_iter().next().unwrap().fail(
            "recovered as a failed attempt after an unclean shutdown",
            repo.clock.now(),
        ) {
            TaskFailureOutcome::Retry(_) => panic!("expected a dead-letter outcome"),
            TaskFailureOutcome::DeadLetter(dead) => repo.dead_letter(&dead).unwrap(),
        }

        assert!(repo.find(id).unwrap().is_none());
        let conn = repo.conn.lock().unwrap();
        let (original_task_id, retries): (i64, i64) = conn
            .query_row(
                "SELECT original_task_id, retries FROM tasks_dead_letter",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(original_task_id, id);
        assert_eq!(retries, 5);
    }
}
