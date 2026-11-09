use crate::domain::task::{DeadLetteredTask, ScheduledTask, Task, TaskStatus};
use crate::domain::video::VideoRecordId;
use crate::infrastructure::shared::sqlite_connection::Database;
use crate::infrastructure::shared::system_clock::Clock;
use anyhow::Context;
use chrono::{DateTime, Utc};
#[cfg(test)]
use rusqlite::Connection;
use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Map, Value};
use std::sync::Arc;
use tracing::info;

pub trait TaskRepository: Send + Sync {
    /// Adds the task, unless it has an exclusivity key and a task of the same
    /// type with the same key is already pending, or running unless
    /// `Task::dedupes_running` says otherwise (then a no-op).
    fn schedule(&self, task: &Task, run_at: DateTime<Utc>) -> anyhow::Result<()>;
    /// Atomically flips a pending task to running; `None` if it was no
    /// longer pending.
    fn claim(&self, id: i64, now: DateTime<Utc>) -> anyhow::Result<Option<ScheduledTask>>;
    /// Tasks ready to run now.
    fn list_eligible(&self) -> anyhow::Result<Vec<ScheduledTask>>;
    /// Every task still `running` from a previous, interrupted process.
    fn list_running(&self) -> anyhow::Result<Vec<ScheduledTask>>;
    /// Every task that has not yet completed (`pending` or `running`),
    /// regardless of scheduled run time.
    fn list_non_completed(&self) -> anyhow::Result<Vec<ScheduledTask>>;
    /// The pending task with this type and payload, if any.
    fn find_pending(&self, task: &Task) -> anyhow::Result<Option<ScheduledTask>>;
    /// The pending or running task of `task_type` for `video_id`, if any.
    fn find_non_completed_for_video(
        &self,
        task_type: &str,
        video_id: &VideoRecordId,
    ) -> anyhow::Result<Option<ScheduledTask>>;
    fn update(&self, task: &ScheduledTask) -> anyhow::Result<()>;
    fn delete(&self, id: i64) -> anyhow::Result<()>;
    /// Atomically inserts `task` into the dead-letter table and deletes the
    /// original row.
    fn dead_letter(&self, task: &DeadLetteredTask) -> anyhow::Result<()>;
}

const SELECT_COLUMNS: &str =
    "id, task_type, payload, status, retries, run_at, created_at, updated_at, last_error";

pub struct SqliteTaskRepository {
    db: Database,
    clock: Arc<dyn Clock>,
}

impl SqliteTaskRepository {
    pub fn new(db: Database, clock: Arc<dyn Clock>) -> Self {
        Self { db, clock }
    }

    fn list_where(&self, predicate: &str) -> anyhow::Result<Vec<ScheduledTask>> {
        let conn = self.db.read()?;
        let query = format!("SELECT {SELECT_COLUMNS} FROM tasks WHERE {predicate} ORDER BY id ASC");
        let mut stmt = conn
            .prepare(&query)
            .inspect_err(|e| tracing::error!(error = %e, "failed to prepare tasks query"))
            .context("failed to prepare tasks query")?;
        let rows = stmt
            .query_map([], row_to_scheduled_task)
            .inspect_err(|e| tracing::error!(error = %e, "failed to list tasks"))
            .context("failed to list tasks")?;
        rows.collect::<Result<Vec<_>, _>>()
            .inspect_err(|e| tracing::error!(error = %e, "failed to read task row"))
            .context("failed to read task row")
    }

    #[cfg(test)]
    fn find(&self, id: i64) -> anyhow::Result<Option<ScheduledTask>> {
        let conn = self.db.read()?;
        conn.query_row(
            &format!("SELECT {SELECT_COLUMNS} FROM tasks WHERE id = ?1"),
            params![id],
            row_to_scheduled_task,
        )
        .optional()
        .context("failed to find task")
    }

    /// Every row in the dead-letter table, which `TaskRepository` cannot read.
    #[cfg(test)]
    pub fn list_dead_lettered(&self) -> anyhow::Result<Vec<DeadLetteredTask>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(
                "SELECT original_task_id, task_type, payload, retries, last_error, created_at, failed_at
                 FROM tasks_dead_letter ORDER BY id ASC",
            )
            .context("failed to prepare dead-lettered tasks query")?;
        let rows = stmt
            .query_map([], |row| {
                let created_at: String = row.get(5)?;
                let failed_at: String = row.get(6)?;
                Ok(DeadLetteredTask {
                    original_task_id: row.get(0)?,
                    task: decode_task(row.get(1)?, row.get(2)?),
                    retries: row.get(3)?,
                    last_error: row.get(4)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .unwrap()
                        .with_timezone(&Utc),
                    failed_at: DateTime::parse_from_rfc3339(&failed_at)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })
            .context("failed to list dead-lettered tasks")?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("failed to read dead-lettered task row")
    }
}

impl TaskRepository for SqliteTaskRepository {
    fn schedule(&self, task: &Task, run_at: DateTime<Utc>) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        let payload = encode_payload(task)?;
        let now = self.clock.now().to_rfc3339();
        let inserted = conn
            .execute(
                &format!(
                    "INSERT INTO tasks (task_type, payload, status, retries, run_at, created_at, updated_at, last_error)
                     SELECT ?1, ?2, 'pending', 0, ?3, ?4, ?4, NULL
                     WHERE {}",
                    dedupe_predicate(task)
                ),
                params![task.task_type(), payload, run_at.to_rfc3339(), now],
            )
            .inspect_err(|e| {
                tracing::error!(task_type = task.task_type(), error = %e, "failed to schedule task")
            })
            .context("failed to schedule task")?;
        if inserted == 0 {
            info!(
                task_type = task.task_type(),
                "task already pending or running for this video, not scheduling a duplicate"
            );
            return Ok(());
        }
        info!(
            task_id = conn.last_insert_rowid(),
            task_type = task.task_type(),
            run_at = %run_at,
            "scheduled task"
        );
        Ok(())
    }

    fn claim(&self, id: i64, now: DateTime<Utc>) -> anyhow::Result<Option<ScheduledTask>> {
        let conn = self.db.write()?;
        conn.query_row(
            &format!(
                "UPDATE tasks SET status = 'running', updated_at = ?2 WHERE id = ?1 AND status = 'pending' RETURNING {SELECT_COLUMNS}"
            ),
            params![id, now.to_rfc3339()],
            row_to_scheduled_task,
        )
        .optional()
        .inspect_err(|e| tracing::error!(task_id = id, error = %e, "failed to claim task"))
        .context("failed to claim task")
    }

    fn list_eligible(&self) -> anyhow::Result<Vec<ScheduledTask>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {SELECT_COLUMNS} FROM tasks WHERE status = 'pending' AND run_at <= ?1 ORDER BY run_at ASC, id ASC"
            ))
            .inspect_err(|e| {
                tracing::error!(error = %e, "failed to prepare list-eligible-tasks query")
            })
            .context("failed to prepare list-eligible-tasks query")?;
        let rows = stmt
            .query_map(
                params![self.clock.now().to_rfc3339()],
                row_to_scheduled_task,
            )
            .inspect_err(|e| tracing::error!(error = %e, "failed to list eligible tasks"))
            .context("failed to list eligible tasks")?;
        rows.collect::<Result<Vec<_>, _>>()
            .inspect_err(|e| tracing::error!(error = %e, "failed to read eligible task row"))
            .context("failed to read eligible task row")
    }

    fn list_running(&self) -> anyhow::Result<Vec<ScheduledTask>> {
        self.list_where("status = 'running'")
    }

    fn list_non_completed(&self) -> anyhow::Result<Vec<ScheduledTask>> {
        self.list_where("status IN ('pending', 'running')")
    }

    fn find_pending(&self, task: &Task) -> anyhow::Result<Option<ScheduledTask>> {
        let conn = self.db.read()?;
        conn.query_row(
            &format!("SELECT {SELECT_COLUMNS} FROM tasks WHERE task_type = ?1 AND payload = ?2 AND status = 'pending' ORDER BY id LIMIT 1"),
            params![task.task_type(), encode_payload(task)?],
            row_to_scheduled_task,
        )
        .optional()
        .inspect_err(|e| tracing::error!(error = %e, "failed to find pending task"))
        .context("failed to find pending task")
    }

    fn find_non_completed_for_video(
        &self,
        task_type: &str,
        video_id: &VideoRecordId,
    ) -> anyhow::Result<Option<ScheduledTask>> {
        let conn = self.db.read()?;
        conn.query_row(
            &format!("SELECT {SELECT_COLUMNS} FROM tasks WHERE task_type = ?1 AND json_extract(payload, '$.video_id') = ?2 AND status IN ('pending', 'running') ORDER BY id LIMIT 1"),
            params![task_type, video_id.as_str()],
            row_to_scheduled_task,
        )
        .optional()
        .inspect_err(|e| tracing::error!(error = %e, "failed to find non-completed task of video"))
        .context("failed to find non-completed task of video")
    }

    fn update(&self, task: &ScheduledTask) -> anyhow::Result<()> {
        let conn = self.db.write()?;
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
        .inspect_err(|e| tracing::error!(task_id = task.id, error = %e, "failed to update task"))
        .context("failed to update task")?;
        Ok(())
    }

    fn delete(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])
            .inspect_err(|e| tracing::error!(task_id = id, error = %e, "failed to delete task"))
            .context("failed to delete task")?;
        Ok(())
    }

    fn dead_letter(&self, task: &DeadLetteredTask) -> anyhow::Result<()> {
        let mut conn = self.db.write()?;
        let tx = conn
            .transaction()
            .inspect_err(|e| {
                tracing::error!(
                    task_id = task.original_task_id,
                    error = %e,
                    "failed to start dead-letter transaction"
                )
            })
            .context("failed to start dead-letter transaction")?;
        tx.execute(
            "INSERT INTO tasks_dead_letter (original_task_id, task_type, payload, retries, last_error, created_at, failed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                task.original_task_id,
                task.task.task_type(),
                encode_payload(&task.task)?,
                task.retries,
                task.last_error,
                task.created_at.to_rfc3339(),
                task.failed_at.to_rfc3339(),
            ],
        )
        .inspect_err(|e| {
            tracing::error!(
                task_id = task.original_task_id,
                error = %e,
                "failed to insert task into dead letter"
            )
        })
        .context("failed to insert task into dead letter")?;
        tx.execute(
            "DELETE FROM tasks WHERE id = ?1",
            params![task.original_task_id],
        )
        .inspect_err(|e| {
            tracing::error!(
                task_id = task.original_task_id,
                error = %e,
                "failed to delete task after moving to dead letter"
            )
        })
        .context("failed to delete task after moving to dead letter")?;
        tx.commit()
            .inspect_err(|e| {
                tracing::error!(
                    task_id = task.original_task_id,
                    error = %e,
                    "failed to commit dead-letter transaction"
                )
            })
            .context("failed to commit dead-letter transaction")?;
        Ok(())
    }
}

fn row_to_scheduled_task(row: &Row) -> rusqlite::Result<ScheduledTask> {
    let status: String = row.get(3)?;
    let run_at: String = row.get(5)?;
    let created_at: String = row.get(6)?;
    let updated_at: String = row.get(7)?;
    Ok(ScheduledTask {
        id: row.get(0)?,
        task: decode_task(row.get(1)?, row.get(2)?),
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

/// The `WHERE` clause guarding `schedule`'s insert: a task with an
/// exclusivity key is only added if no task of its type for the same video
/// is pending, or also running when `Task::dedupes_running`; any other task
/// is always added.
fn dedupe_predicate(task: &Task) -> &'static str {
    match task.exclusivity_key() {
        Some(_) if task.dedupes_running() => {
            "NOT EXISTS (SELECT 1 FROM tasks WHERE task_type = ?1
               AND json_extract(payload, '$.video_id') = json_extract(?2, '$.video_id')
               AND status IN ('pending', 'running'))"
        }
        Some(_) => {
            "NOT EXISTS (SELECT 1 FROM tasks WHERE task_type = ?1
               AND json_extract(payload, '$.video_id') = json_extract(?2, '$.video_id')
               AND status = 'pending')"
        }
        None => "1",
    }
}

/// Decodes a stored row's `task_type` and `payload` columns into its task:
/// the payload's fields plus `task_type` as the serde tag. A row this
/// version can't decode becomes `Task::Unreadable`, holding the columns
/// verbatim, rather than failing the whole read.
fn decode_task(task_type: String, payload: String) -> Task {
    serde_json::from_str::<Map<String, Value>>(&payload)
        .and_then(|mut fields| {
            fields.insert("task_type".to_string(), Value::String(task_type.clone()));
            serde_json::from_value(Value::Object(fields))
        })
        .unwrap_or_else(|e| Task::Unreadable {
            task_type,
            payload,
            error: e.to_string(),
        })
}

/// The `payload` column of `task`: its fields as a JSON object with keys in
/// alphabetical order, without the `task_type` tag. An unreadable task
/// keeps the payload it was read with.
fn encode_payload(task: &Task) -> anyhow::Result<String> {
    if let Task::Unreadable { payload, .. } = task {
        return Ok(payload.clone());
    }
    let mut fields = match serde_json::to_value(task).context("failed to encode task")? {
        Value::Object(fields) => fields,
        other => anyhow::bail!("task encoded as a non-object: {other}"),
    };
    fields.remove("task_type");
    Ok(Value::Object(fields).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::TaskFailureOutcome;
    use crate::infrastructure::shared::system_clock::FixedClock;

    fn repo_with_clock(now: DateTime<Utc>) -> SqliteTaskRepository {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::shared::sqlite_migrations::apply(&mut conn).unwrap();
        SqliteTaskRepository::new(Database::single(conn), Arc::new(FixedClock(now)))
    }

    fn repo() -> SqliteTaskRepository {
        repo_with_clock(DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap())
    }

    fn task() -> Task {
        Task::ReconcilePlaylist {
            playlist_id: "PL1".to_string(),
        }
    }

    fn pending_task(
        id: i64,
        task: &Task,
        run_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> ScheduledTask {
        ScheduledTask {
            id,
            task: task.clone(),
            status: TaskStatus::Pending,
            retries: 0,
            run_at,
            created_at: now,
            updated_at: now,
            last_error: None,
        }
    }

    const TEST_BASE_RETRY_DELAY_SECONDS: i64 = 150;

    /// Mirrors what `TaskExecutor` does on a dispatch failure: read the task,
    /// let it decide retry vs. dead-letter, then persist that decision.
    fn apply_failure(repo: &SqliteTaskRepository, id: i64, error: &str) {
        let scheduled = repo.find(id).unwrap().unwrap();
        match scheduled.fail(error, repo.clock.now(), TEST_BASE_RETRY_DELAY_SECONDS) {
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
        assert_eq!(eligible[0].task, task());
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
    fn it_should_list_eligible_tasks_by_run_time_regardless_of_scheduling_order() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        let later_due = now - chrono::Duration::seconds(1);
        let earlier_due = now - chrono::Duration::seconds(60);
        let reconcile = task();
        let download = Task::DownloadVideo {
            video_id: "rec1".to_string(),
            quality: "high".to_string(),
            output_dir: "/videos/my-playlist".to_string(),
        };

        repo.schedule(&reconcile, later_due).unwrap();
        repo.schedule(&download, earlier_due).unwrap();

        assert_eq!(
            repo.list_eligible().unwrap(),
            vec![
                pending_task(2, &download, earlier_due, now),
                pending_task(1, &reconcile, later_due, now),
            ]
        );
    }

    #[test]
    fn it_should_list_eligible_tasks_due_at_the_same_time_in_scheduling_order() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        let first = task();
        let second = Task::ReconcilePlaylist {
            playlist_id: "PL2".to_string(),
        };

        repo.schedule(&first, now).unwrap();
        repo.schedule(&second, now).unwrap();

        assert_eq!(
            repo.list_eligible().unwrap(),
            vec![
                pending_task(1, &first, now, now),
                pending_task(2, &second, now, now),
            ]
        );
    }

    #[test]
    fn it_should_no_longer_list_a_task_as_eligible_once_deleted() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;

        repo.delete(id).unwrap();

        assert!(repo.list_eligible().unwrap().is_empty());
        assert!(repo.find(id).unwrap().is_none());
    }

    #[test]
    fn it_should_create_a_queryable_empty_dead_letter_table() {
        let repo = repo();

        let conn = repo.db.read().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM tasks_dead_letter").unwrap();
        let rows = stmt.query_map([], |_| Ok(())).unwrap();

        assert_eq!(rows.count(), 0);
    }

    #[test]
    fn it_should_no_longer_list_a_running_task_as_eligible() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;
        let scheduled = repo.find(id).unwrap().unwrap();

        repo.update(&scheduled.start(repo.clock.now())).unwrap();

        assert!(repo.list_eligible().unwrap().is_empty());
    }

    #[test]
    fn it_should_drop_the_task_after_the_fifth_failed_attempt() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;

        for _ in 0..5 {
            apply_failure(&repo, id, "boom");
        }

        assert!(repo.list_eligible().unwrap().is_empty());
        assert!(repo.find(id).unwrap().is_none());

        let conn = repo.db.read().unwrap();
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
        assert_eq!(task_type, "reconcile_playlist");
        assert_eq!(payload, r#"{"playlist_id":"PL1"}"#);
        assert_eq!(retries, 5);
        assert_eq!(last_error, "boom");
    }

    #[test]
    fn it_should_reschedule_the_task_after_the_growing_retry_delay_below_the_attempt_limit() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        repo.schedule(&task(), now).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;

        apply_failure(&repo, id, "boom");

        assert!(repo.list_eligible().unwrap().is_empty());
        let found = repo.find(id).unwrap().unwrap();
        assert_eq!(
            found.run_at,
            now + chrono::Duration::seconds(
                crate::domain::task::scheduled_task::retry_delay_seconds(
                    1,
                    TEST_BASE_RETRY_DELAY_SECONDS
                )
            )
        );
        assert_eq!(found.retries, 1);
        assert_eq!(found.last_error, Some("boom".to_string()));
    }

    #[test]
    fn it_should_retry_a_recovered_running_task_when_under_the_attempt_limit() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;
        let scheduled = repo.find(id).unwrap().unwrap();
        repo.update(&scheduled.start(repo.clock.now())).unwrap();

        let running = repo.list_running().unwrap();
        assert_eq!(running.len(), 1);
        match running.into_iter().next().unwrap().fail(
            "recovered as a failed attempt after an unclean shutdown",
            repo.clock.now(),
            TEST_BASE_RETRY_DELAY_SECONDS,
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
        let id = repo.list_eligible().unwrap()[0].id;

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
            TEST_BASE_RETRY_DELAY_SECONDS,
        ) {
            TaskFailureOutcome::Retry(_) => panic!("expected a dead-letter outcome"),
            TaskFailureOutcome::DeadLetter(dead) => repo.dead_letter(&dead).unwrap(),
        }

        assert!(repo.find(id).unwrap().is_none());
        let conn = repo.db.read().unwrap();
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

    #[test]
    fn it_should_list_a_future_pending_task_as_non_completed() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        let future = now + chrono::Duration::seconds(60);
        repo.schedule(&task(), future).unwrap();

        let non_completed = repo.list_non_completed().unwrap();

        assert_eq!(non_completed.len(), 1);
        assert_eq!(non_completed[0].status, TaskStatus::Pending);
        assert_eq!(non_completed[0].run_at, future);
    }

    #[test]
    fn it_should_not_list_a_dead_lettered_task_as_non_completed() {
        let repo = repo();
        repo.schedule(&task(), repo.clock.now()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;

        for _ in 0..5 {
            apply_failure(&repo, id, "boom");
        }

        assert!(repo.list_non_completed().unwrap().is_empty());
    }

    #[test]
    fn it_should_not_schedule_a_duplicate_download_for_the_same_video() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        let download = download_task("rec1");

        repo.schedule(&download, now).unwrap();
        repo.schedule(&download, now).unwrap();

        assert_eq!(
            repo.list_non_completed().unwrap(),
            vec![pending_task(1, &download, now, now)]
        );
    }

    #[test]
    fn it_should_not_schedule_a_duplicate_thumbnail_fetch_for_the_same_video() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        let fetch_thumbnail = Task::FetchThumbnail {
            video_id: "rec1".to_string(),
            output_dir: "/videos/my-playlist".to_string(),
        };

        repo.schedule(&fetch_thumbnail, now).unwrap();
        repo.schedule(&fetch_thumbnail, now).unwrap();

        assert_eq!(
            repo.list_non_completed().unwrap(),
            vec![pending_task(1, &fetch_thumbnail, now, now)]
        );
    }

    #[test]
    fn it_should_not_add_a_video_sync_if_one_is_pending() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        let sync = sync_video_task("rec1");

        repo.schedule(&sync, now).unwrap();
        repo.schedule(&sync, now).unwrap();

        assert_eq!(
            repo.list_non_completed().unwrap(),
            vec![pending_task(1, &sync, now, now)]
        );
    }

    #[test]
    fn it_should_add_a_video_sync_if_the_only_one_is_running() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        let sync = sync_video_task("rec1");
        repo.schedule(&sync, now).unwrap();
        repo.claim(1, now).unwrap();

        repo.schedule(&sync, now).unwrap();

        assert_eq!(
            repo.list_non_completed().unwrap(),
            vec![
                pending_task(1, &sync, now, now).start(now),
                pending_task(2, &sync, now, now),
            ]
        );
    }

    #[test]
    fn it_should_not_add_a_download_if_one_is_running() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        let download = download_task("rec1");
        repo.schedule(&download, now).unwrap();
        repo.claim(1, now).unwrap();

        repo.schedule(&download, now).unwrap();

        assert_eq!(
            repo.list_non_completed().unwrap(),
            vec![pending_task(1, &download, now, now).start(now)]
        );
    }

    #[test]
    fn it_should_schedule_a_download_for_another_video() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);

        repo.schedule(&download_task("rec1"), now).unwrap();
        repo.schedule(&download_task("rec2"), now).unwrap();

        assert_eq!(
            repo.list_non_completed().unwrap(),
            vec![
                pending_task(1, &download_task("rec1"), now, now),
                pending_task(2, &download_task("rec2"), now, now),
            ]
        );
    }

    #[test]
    fn it_should_schedule_duplicates_of_tasks_without_a_video() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);

        repo.schedule(&task(), now).unwrap();
        repo.schedule(&task(), now).unwrap();

        assert_eq!(
            repo.list_non_completed().unwrap(),
            vec![
                pending_task(1, &task(), now, now),
                pending_task(2, &task(), now, now),
            ]
        );
    }

    #[test]
    fn it_should_find_the_pending_task_with_the_same_type_and_payload() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        let run_at = now + chrono::Duration::hours(1);
        repo.schedule(&task(), run_at).unwrap();

        let found = repo.find_pending(&task()).unwrap();

        assert_eq!(found, Some(pending_task(1, &task(), run_at, now)));
    }

    #[test]
    fn it_should_not_find_a_running_or_completed_task_as_pending() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        repo.schedule(&task(), now).unwrap();
        repo.schedule(&task(), now).unwrap();
        repo.claim(1, now).unwrap();
        repo.delete(2).unwrap();

        let found = repo.find_pending(&task()).unwrap();

        assert_eq!(found, None);
    }

    #[test]
    fn it_should_not_find_a_pending_task_of_another_source() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        repo.schedule(&task(), now).unwrap();

        let found = repo
            .find_pending(&Task::ReconcilePlaylist {
                playlist_id: "PL2".to_string(),
            })
            .unwrap();

        assert_eq!(found, None);
    }

    #[test]
    fn it_should_claim_a_pending_task() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let later = now + chrono::Duration::seconds(30);
        let repo = repo_with_clock(now);
        repo.schedule(&task(), now).unwrap();

        let claimed = repo.claim(1, later).unwrap();

        let running = ScheduledTask {
            status: TaskStatus::Running,
            updated_at: later,
            ..pending_task(1, &task(), now, now)
        };
        assert_eq!(claimed, Some(running.clone()));
        assert_eq!(repo.list_non_completed().unwrap(), vec![running]);
    }

    #[test]
    fn it_should_not_claim_a_task_already_running() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let later = now + chrono::Duration::seconds(30);
        let repo = repo_with_clock(now);
        repo.schedule(&task(), now).unwrap();
        repo.update(&pending_task(1, &task(), now, now).start(now))
            .unwrap();

        let claimed = repo.claim(1, later).unwrap();

        assert_eq!(claimed, None);
        assert_eq!(
            repo.list_non_completed().unwrap(),
            vec![pending_task(1, &task(), now, now).start(now)]
        );
    }

    #[test]
    fn it_should_find_a_running_task_of_a_video() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        repo.schedule(&sync_video_task("v1"), now).unwrap();
        repo.schedule(&download_task("v1"), now).unwrap();
        repo.claim(2, now).unwrap();

        let found = repo
            .find_non_completed_for_video("download_video", &VideoRecordId::new("v1").unwrap())
            .unwrap();

        assert_eq!(
            found,
            Some(pending_task(2, &download_task("v1"), now, now).start(now))
        );
    }

    #[test]
    fn it_should_not_find_a_completed_task_of_a_video() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        repo.schedule(&download_task("v1"), now).unwrap();
        repo.delete(1).unwrap();
        repo.schedule(&download_task("v2"), now).unwrap();

        let found = repo
            .find_non_completed_for_video("download_video", &VideoRecordId::new("v1").unwrap())
            .unwrap();

        assert_eq!(found, None);
    }

    #[test]
    fn it_should_not_queue_a_second_summary() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let repo = repo_with_clock(now);
        let task = Task::SummarizeVideo {
            video_id: "rec1".to_string(),
        };

        repo.schedule(&task, now).unwrap();
        repo.schedule(&task, now).unwrap();

        assert_eq!(
            repo.list_non_completed().unwrap(),
            vec![pending_task(1, &task, now, now)]
        );
    }

    #[test]
    fn it_should_store_every_task_in_its_existing_format() {
        let repo = repo();
        let now = repo.clock.now();

        every_task()
            .iter()
            .try_for_each(|task| repo.schedule(task, now))
            .unwrap();

        assert_eq!(
            stored_rows(&repo),
            every_stored_row()
                .iter()
                .map(|(task_type, payload)| (task_type.to_string(), payload.to_string()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn it_should_read_back_every_task_stored_in_the_existing_format() {
        let repo = repo();
        let now = repo.clock.now();
        every_stored_row()
            .iter()
            .for_each(|(task_type, payload)| insert_row(&repo, task_type, payload));

        let tasks: Vec<Task> = repo
            .list_non_completed()
            .unwrap()
            .into_iter()
            .map(|scheduled| scheduled.task)
            .collect();

        assert_eq!(tasks, every_task());
        assert_eq!(
            repo.list_eligible().unwrap(),
            every_task()
                .iter()
                .enumerate()
                .map(|(index, task)| pending_task(index as i64 + 1, task, now, now))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn it_should_read_a_video_file_deletion_queued_without_a_subtitle_filename() {
        let repo = repo();
        insert_row(
            &repo,
            "delete_video_file",
            r#"{"filename":"My Video.mp4","output_dir":"/videos/music","thumbnail_filename":null}"#,
        );

        let tasks: Vec<Task> = repo
            .list_non_completed()
            .unwrap()
            .into_iter()
            .map(|scheduled| scheduled.task)
            .collect();

        assert_eq!(
            tasks,
            vec![Task::DeleteVideoFile {
                filename: Some("My Video.mp4".to_string()),
                thumbnail_filename: None,
                subtitle_filename: None,
                output_dir: "/videos/music".to_string(),
            }]
        );
    }

    #[test]
    fn it_should_read_a_row_it_cannot_decode_as_unreadable() {
        let repo = repo();
        insert_row(&repo, "reconcile_plex_collections", "{}");
        insert_row(&repo, "download_video", r#"{"video_id":"rec1"}"#);
        insert_row(&repo, "sync_video", "not json");

        let tasks: Vec<Task> = repo
            .list_non_completed()
            .unwrap()
            .into_iter()
            .map(|scheduled| scheduled.task)
            .collect();

        assert_eq!(
            tasks,
            vec![
                Task::Unreadable {
                    task_type: "reconcile_plex_collections".to_string(),
                    payload: "{}".to_string(),
                    error: "unknown variant `reconcile_plex_collections`, expected one of \
                            `reconcile_playlist`, `reconcile_channel`, `download_video`, \
                            `delete_video_file`, `delete_playlist_files`, `delete_channel_files`, \
                            `update_ytdlp`, `fetch_thumbnail`, `fetch_subtitles`, `sync_video`, \
                            `sync_plex_labels`, `fetch_chapters`, `upgrade_quality`, \
                            `summarize_video`, `categorize_video`"
                        .to_string(),
                },
                Task::Unreadable {
                    task_type: "download_video".to_string(),
                    payload: r#"{"video_id":"rec1"}"#.to_string(),
                    error: "missing field `quality`".to_string(),
                },
                Task::Unreadable {
                    task_type: "sync_video".to_string(),
                    payload: "not json".to_string(),
                    error: "expected ident at line 1 column 2".to_string(),
                },
            ]
        );
    }

    #[test]
    fn it_should_dead_letter_an_unreadable_task_verbatim() {
        let repo = repo();
        insert_row(&repo, "reconcile_plex_collections", r#"{"b":1,"a":2}"#);

        for _ in 0..5 {
            apply_failure(&repo, 1, "boom");
        }

        let conn = repo.db.read().unwrap();
        let row: (String, String) = conn
            .query_row(
                "SELECT task_type, payload FROM tasks_dead_letter",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            row,
            (
                "reconcile_plex_collections".to_string(),
                r#"{"b":1,"a":2}"#.to_string()
            )
        );
    }

    /// Inserts a pending row as an earlier version stored it.
    fn insert_row(repo: &SqliteTaskRepository, task_type: &str, payload: &str) {
        let now = repo.clock.now().to_rfc3339();
        repo.db
            .write()
            .unwrap()
            .execute(
                "INSERT INTO tasks (task_type, payload, status, retries, run_at, created_at, updated_at, last_error)
                 VALUES (?1, ?2, 'pending', 0, ?3, ?3, ?3, NULL)",
                params![task_type, payload, now],
            )
            .unwrap();
    }

    fn stored_rows(repo: &SqliteTaskRepository) -> Vec<(String, String)> {
        let conn = repo.db.read().unwrap();
        let mut stmt = conn
            .prepare("SELECT task_type, payload FROM tasks ORDER BY id")
            .unwrap();
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    /// One task of every type, matching `every_stored_row`.
    fn every_task() -> Vec<Task> {
        vec![
            Task::ReconcilePlaylist {
                playlist_id: "PL1".to_string(),
            },
            Task::ReconcileChannel {
                channel_id: "@somechannel".to_string(),
            },
            download_task("rec1"),
            Task::DeleteVideoFile {
                filename: Some("My Video.mp4".to_string()),
                thumbnail_filename: None,
                subtitle_filename: Some("My Video.en.srt".to_string()),
                output_dir: "/videos/my-playlist".to_string(),
            },
            Task::DeletePlaylistFiles {
                playlist_id: "PL1".to_string(),
                path: "music/chill".to_string(),
            },
            Task::DeleteChannelFiles {
                channel_id: "@somechannel".to_string(),
                path: "creators/somechannel".to_string(),
            },
            Task::UpdateYtdlp,
            Task::FetchThumbnail {
                video_id: "rec1".to_string(),
                output_dir: "/videos/my-playlist".to_string(),
            },
            Task::FetchSubtitles {
                video_id: "rec1".to_string(),
                output_dir: "/videos/my-playlist".to_string(),
            },
            sync_video_task("rec1"),
            Task::SyncPlexLabels,
            Task::FetchChapters {
                video_id: "rec1".to_string(),
            },
            Task::UpgradeQuality {
                video_id: "rec1".to_string(),
                quality: "mid".to_string(),
                output_dir: "/videos/my-playlist".to_string(),
            },
            Task::SummarizeVideo {
                video_id: "rec1".to_string(),
            },
            Task::CategorizeVideo {
                video_id: "rec1".to_string(),
            },
        ]
    }

    /// The exact `task_type` and `payload` columns earlier versions stored
    /// for `every_task`, which existing databases hold.
    fn every_stored_row() -> Vec<(&'static str, &'static str)> {
        vec![
            ("reconcile_playlist", r#"{"playlist_id":"PL1"}"#),
            ("reconcile_channel", r#"{"channel_id":"@somechannel"}"#),
            (
                "download_video",
                r#"{"output_dir":"/videos/my-playlist","quality":"high","video_id":"rec1"}"#,
            ),
            (
                "delete_video_file",
                r#"{"filename":"My Video.mp4","output_dir":"/videos/my-playlist","subtitle_filename":"My Video.en.srt","thumbnail_filename":null}"#,
            ),
            (
                "delete_playlist_files",
                r#"{"path":"music/chill","playlist_id":"PL1"}"#,
            ),
            (
                "delete_channel_files",
                r#"{"channel_id":"@somechannel","path":"creators/somechannel"}"#,
            ),
            ("update_ytdlp", "{}"),
            (
                "fetch_thumbnail",
                r#"{"output_dir":"/videos/my-playlist","video_id":"rec1"}"#,
            ),
            (
                "fetch_subtitles",
                r#"{"output_dir":"/videos/my-playlist","video_id":"rec1"}"#,
            ),
            ("sync_video", r#"{"video_id":"rec1"}"#),
            ("sync_plex_labels", "{}"),
            ("fetch_chapters", r#"{"video_id":"rec1"}"#),
            (
                "upgrade_quality",
                r#"{"output_dir":"/videos/my-playlist","quality":"mid","video_id":"rec1"}"#,
            ),
            ("summarize_video", r#"{"video_id":"rec1"}"#),
            ("categorize_video", r#"{"video_id":"rec1"}"#),
        ]
    }

    fn download_task(video_id: &str) -> Task {
        Task::DownloadVideo {
            video_id: video_id.to_string(),
            quality: "high".to_string(),
            output_dir: "/videos/my-playlist".to_string(),
        }
    }

    fn sync_video_task(video_id: &str) -> Task {
        Task::SyncVideo {
            video_id: video_id.to_string(),
        }
    }
}
