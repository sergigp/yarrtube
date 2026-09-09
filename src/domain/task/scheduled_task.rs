use chrono::{DateTime, Duration, Utc};

const MAX_ATTEMPTS: i64 = 5;
const RETRY_DELAY_SECONDS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
        }
    }

    pub fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            other => Err(anyhow::anyhow!("unknown task status '{other}'")),
        }
    }
}

/// A persisted task's full lifecycle state. `TaskRepository` persists it
/// verbatim; every retry/backoff/dead-letter decision is made here via
/// `start`/`fail`, never inside the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledTask {
    pub id: i64,
    pub task_type: String,
    pub payload: String,
    pub status: TaskStatus,
    pub retries: i64,
    pub run_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_error: Option<String>,
}

/// A task that exhausted its retry budget, shaped for the dead-letter table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadLetteredTask {
    pub original_task_id: i64,
    pub task_type: String,
    pub payload: String,
    pub retries: i64,
    pub last_error: String,
    pub created_at: DateTime<Utc>,
    pub failed_at: DateTime<Utc>,
}

pub enum TaskFailureOutcome {
    Retry(ScheduledTask),
    DeadLetter(DeadLetteredTask),
}

impl ScheduledTask {
    pub fn start(self, now: DateTime<Utc>) -> Self {
        Self {
            status: TaskStatus::Running,
            updated_at: now,
            ..self
        }
    }

    /// Whether a failure of this attempt would exhaust the retry budget
    /// (`fail` would dead-letter rather than retry).
    pub fn is_last_attempt(&self) -> bool {
        self.retries + 1 >= MAX_ATTEMPTS
    }

    pub fn fail(self, error: impl Into<String>, now: DateTime<Utc>) -> TaskFailureOutcome {
        let error = error.into();
        let retries = self.retries + 1;

        if retries >= MAX_ATTEMPTS {
            TaskFailureOutcome::DeadLetter(DeadLetteredTask {
                original_task_id: self.id,
                task_type: self.task_type,
                payload: self.payload,
                retries,
                last_error: error,
                created_at: self.created_at,
                failed_at: now,
            })
        } else {
            TaskFailureOutcome::Retry(Self {
                status: TaskStatus::Pending,
                retries,
                run_at: now + Duration::seconds(RETRY_DELAY_SECONDS),
                updated_at: now,
                last_error: Some(error),
                ..self
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task_with_retries(retries: i64) -> ScheduledTask {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        ScheduledTask {
            id: 1,
            task_type: "sync_playlist".to_string(),
            payload: "{}".to_string(),
            status: TaskStatus::Pending,
            retries,
            run_at: now,
            created_at: now,
            updated_at: now,
            last_error: None,
        }
    }

    #[test]
    fn it_should_transition_to_running_when_started() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_100, 0).unwrap();

        let started = task_with_retries(0).start(now);

        assert_eq!(started.status, TaskStatus::Running);
        assert_eq!(started.updated_at, now);
    }

    #[test]
    fn it_should_retry_with_incremented_retries_and_a_delayed_run_at_when_below_the_attempt_limit()
    {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();

        let outcome = task_with_retries(3).fail("boom", now);

        match outcome {
            TaskFailureOutcome::Retry(retried) => {
                assert_eq!(retried.status, TaskStatus::Pending);
                assert_eq!(retried.retries, 4);
                assert_eq!(retried.run_at, now + Duration::seconds(RETRY_DELAY_SECONDS));
                assert_eq!(retried.updated_at, now);
                assert_eq!(retried.last_error, Some("boom".to_string()));
            }
            TaskFailureOutcome::DeadLetter(_) => panic!("expected a retry outcome"),
        }
    }

    #[test]
    fn it_should_dead_letter_when_the_fifth_attempt_fails() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();

        let outcome = task_with_retries(4).fail("boom", now);

        match outcome {
            TaskFailureOutcome::DeadLetter(dead) => {
                assert_eq!(dead.original_task_id, 1);
                assert_eq!(dead.task_type, "sync_playlist");
                assert_eq!(dead.retries, 5);
                assert_eq!(dead.last_error, "boom");
                assert_eq!(dead.failed_at, now);
            }
            TaskFailureOutcome::Retry(_) => panic!("expected a dead-letter outcome"),
        }
    }

    #[test]
    fn it_should_not_report_the_last_attempt_when_retries_are_left() {
        assert!(!task_with_retries(3).is_last_attempt());
    }

    #[test]
    fn it_should_report_the_last_attempt_when_the_next_failure_would_dead_letter() {
        assert!(task_with_retries(4).is_last_attempt());
    }

    #[test]
    fn it_should_round_trip_status_through_its_string_representation() {
        assert_eq!(TaskStatus::parse("pending").unwrap(), TaskStatus::Pending);
        assert_eq!(TaskStatus::parse("running").unwrap(), TaskStatus::Running);
        assert!(TaskStatus::parse("bogus").is_err());
        assert_eq!(TaskStatus::Pending.as_str(), "pending");
        assert_eq!(TaskStatus::Running.as_str(), "running");
    }
}
