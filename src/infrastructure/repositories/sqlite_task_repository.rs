use crate::domain::task::Task;
use crate::infrastructure::repositories::system_clock::Clock;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

const MAX_ATTEMPTS: i64 = 5;
const RETRY_DELAY_SECONDS: i64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedTask {
    pub id: i64,
    pub task_type: String,
    pub payload: String,
}

pub trait TaskRepository: Send + Sync {
    fn schedule(&self, task: &Task, run_at: DateTime<Utc>) -> anyhow::Result<()>;
    fn list_eligible(&self) -> anyhow::Result<Vec<PersistedTask>>;
    fn mark_running(&self, id: i64) -> anyhow::Result<()>;
    fn mark_done(&self, id: i64) -> anyhow::Result<()>;
    fn mark_failed_or_retry(&self, id: i64, error: &str) -> anyhow::Result<()>;
    /// Recovers every task still `running` from a previous, interrupted
    /// process, applying the same failed-attempt handling as
    /// `mark_failed_or_retry` to each one.
    fn recover_running(&self) -> anyhow::Result<()>;
}

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

    fn apply_failed_attempt(
        &self,
        conn: &mut Connection,
        id: i64,
        error: &str,
    ) -> anyhow::Result<()> {
        let (task_type, payload, created_at, retries): (String, String, String, i64) = conn
            .query_row(
                "SELECT task_type, payload, created_at, retries FROM tasks WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .context("failed to read task before failure")?;
        let retries = retries + 1;
        let now = self.clock.now();

        if retries >= MAX_ATTEMPTS {
            let tx = conn
                .transaction()
                .context("failed to start dead-letter transaction")?;
            tx.execute(
                "INSERT INTO tasks_dead_letter (original_task_id, task_type, payload, retries, last_error, created_at, failed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![id, task_type, payload, retries, error, created_at, now.to_rfc3339()],
            )

            .context("failed to insert task into dead letter")?;
            tx.execute("DELETE FROM tasks WHERE id = ?1", params![id])
                .context("failed to delete task after moving to dead letter")?;
            tx.commit()
                .context("failed to commit dead-letter transaction")?;
            error!(task_id = id, retries, error, "task failed permanently");
        } else {
            let run_at = now + chrono::Duration::seconds(RETRY_DELAY_SECONDS);
            conn.execute(
                "UPDATE tasks SET status = 'pending', retries = ?2, run_at = ?3, updated_at = ?4, last_error = ?5 WHERE id = ?1",
                params![id, retries, run_at.to_rfc3339(), now.to_rfc3339(), error],
            )
            .context("failed to reschedule task after failure")?;
            warn!(
                task_id = id,
                retries,
                max_attempts = MAX_ATTEMPTS,
                run_at = %run_at,
                error,
                "task failed, retrying"
            );
        }
        Ok(())
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

    fn list_eligible(&self) -> anyhow::Result<Vec<PersistedTask>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_type, payload FROM tasks
                 WHERE status = 'pending' AND run_at <= ?1
                 ORDER BY id ASC",
            )
            .context("failed to prepare list-eligible-tasks query")?;
        let rows = stmt
            .query_map(params![self.clock.now().to_rfc3339()], |row| {
                Ok(PersistedTask {
                    id: row.get(0)?,
                    task_type: row.get(1)?,
                    payload: row.get(2)?,
                })
            })
            .context("failed to list eligible tasks")?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("failed to read task row")
    }

    fn mark_running(&self, id: i64) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "UPDATE tasks SET status = 'running', updated_at = ?2 WHERE id = ?1",
            params![id, self.clock.now().to_rfc3339()],
        )
        .context("failed to mark task running")?;
        Ok(())
    }

    fn mark_done(&self, id: i64) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])
            .context("failed to mark task done")?;
        info!(task_id = id, "task done");
        Ok(())
    }

    fn mark_failed_or_retry(&self, id: i64, error: &str) -> anyhow::Result<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        self.apply_failed_attempt(&mut conn, id, error)
    }

    fn recover_running(&self) -> anyhow::Result<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let running_ids: Vec<i64> = {
            let mut stmt = conn
                .prepare("SELECT id FROM tasks WHERE status = 'running'")
                .context("failed to prepare running-tasks query")?;
            let rows = stmt
                .query_map([], |row| row.get(0))
                .context("failed to list running tasks")?;
            rows.collect::<Result<Vec<_>, _>>()
                .context("failed to read running task row")?
        };

        for id in running_ids {
            warn!(
                task_id = id,
                "recovering task left running after an unclean shutdown"
            );
            self.apply_failed_attempt(
                &mut conn,
                id,
                "recovered as a failed attempt after an unclean shutdown",
            )?;
        }
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

    fn list_eligible(&self) -> anyhow::Result<Vec<PersistedTask>> {
        Ok(Vec::new())
    }

    fn mark_running(&self, _id: i64) -> anyhow::Result<()> {
        Ok(())
    }

    fn mark_done(&self, _id: i64) -> anyhow::Result<()> {
        Ok(())
    }

    fn mark_failed_or_retry(&self, _id: i64, _error: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn recover_running(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::system_clock::FixedClock;
    use rusqlite::OptionalExtension;

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

    #[test]
    fn it_should_list_a_task_scheduled_for_now_as_eligible() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);

        repo.schedule(&task(), now).unwrap();

        let eligible = repo.list_eligible().unwrap();
        assert_eq!(eligible.len(), 1);
        assert_eq!(eligible[0].task_type, "sync_playlist");
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
    fn it_should_no_longer_list_a_task_as_eligible_once_marked_done() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;

        repo.mark_done(id).unwrap();

        assert!(repo.list_eligible().unwrap().is_empty());
        let conn = repo.conn.lock().unwrap();
        let found: Option<i64> = conn
            .query_row("SELECT id FROM tasks WHERE id = ?1", params![id], |row| {
                row.get(0)
            })
            .optional()
            .unwrap();
        assert!(found.is_none());
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
        let id = repo.list_eligible().unwrap()[0].id;

        repo.mark_running(id).unwrap();

        assert!(repo.list_eligible().unwrap().is_empty());
    }

    #[test]
    fn it_should_drop_the_task_after_the_fifth_failed_attempt() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;

        for _ in 0..5 {
            repo.mark_failed_or_retry(id, "boom").unwrap();
        }

        assert!(repo.list_eligible().unwrap().is_empty());
        let conn = repo.conn.lock().unwrap();
        let found: Option<i64> = conn
            .query_row("SELECT id FROM tasks WHERE id = ?1", params![id], |row| {
                row.get(0)
            })
            .optional()
            .unwrap();
        assert!(found.is_none());

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
        let id = repo.list_eligible().unwrap()[0].id;

        repo.mark_failed_or_retry(id, "boom").unwrap();

        assert!(repo.list_eligible().unwrap().is_empty());

        let conn = repo.conn.lock().unwrap();
        let run_at: String = conn
            .query_row("SELECT run_at FROM tasks WHERE id = ?1", params![id], |r| {
                r.get(0)
            })
            .unwrap();
        drop(conn);
        let run_at = DateTime::parse_from_rfc3339(&run_at).unwrap();
        assert_eq!(
            run_at.with_timezone(&Utc),
            now + chrono::Duration::seconds(RETRY_DELAY_SECONDS)
        );
    }

    #[test]
    fn it_should_retry_a_recovered_running_task_when_under_the_attempt_limit() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;
        repo.mark_running(id).unwrap();

        repo.recover_running().unwrap();

        let conn = repo.conn.lock().unwrap();
        let (status, retries): (String, i64) = conn
            .query_row(
                "SELECT status, retries FROM tasks WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "pending");
        assert_eq!(retries, 1);
    }

    #[test]
    fn it_should_drop_a_recovered_running_task_once_the_attempt_limit_is_exceeded() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;

        for _ in 0..4 {
            repo.mark_failed_or_retry(id, "boom").unwrap();
        }
        repo.mark_running(id).unwrap();

        repo.recover_running().unwrap();

        let conn = repo.conn.lock().unwrap();
        let found: Option<i64> = conn
            .query_row("SELECT id FROM tasks WHERE id = ?1", params![id], |r| {
                r.get(0)
            })
            .optional()
            .unwrap();
        assert!(found.is_none());

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
