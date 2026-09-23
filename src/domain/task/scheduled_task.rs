use chrono::{DateTime, Duration, Utc};

const MAX_ATTEMPTS: i64 = 5;

/// Retry delay grows geometrically with the post-increment retry count:
/// `base_delay_seconds * RETRY_DELAY_MULTIPLIER.pow(retries)`. With the
/// default base delay, a task's first 4 failures (the 5th dead-letters
/// instead of retrying) span about 5 hours total end-to-end (~7.5min,
/// ~22.5min, ~1.1h, ~3.4h between successive attempts) — wide enough for the
/// hourly `yt-dlp` self-update loop to realistically fix a systemic problem
/// before `download_video`'s attempts are exhausted. `base_delay_seconds` is
/// applied uniformly to every task type (see design.md's "Retry-count-based
/// delay, applied uniformly" decision); only it is configurable, the
/// multiplier stays fixed.
const RETRY_DELAY_MULTIPLIER: i64 = 3;

pub(crate) fn retry_delay_seconds(retries: i64, base_delay_seconds: i64) -> i64 {
    base_delay_seconds * RETRY_DELAY_MULTIPLIER.pow(retries as u32)
}

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

    pub fn fail(
        self,
        error: impl Into<String>,
        now: DateTime<Utc>,
        base_retry_delay_seconds: i64,
    ) -> TaskFailureOutcome {
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
                run_at: now
                    + Duration::seconds(retry_delay_seconds(retries, base_retry_delay_seconds)),
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

    const TEST_BASE_RETRY_DELAY_SECONDS: i64 = 150;

    fn task_with_retries(retries: i64) -> ScheduledTask {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        ScheduledTask {
            id: 1,
            task_type: "reconcile_playlist".to_string(),
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

        let outcome = task_with_retries(3).fail("boom", now, TEST_BASE_RETRY_DELAY_SECONDS);

        match outcome {
            TaskFailureOutcome::Retry(retried) => {
                assert_eq!(retried.status, TaskStatus::Pending);
                assert_eq!(retried.retries, 4);
                assert_eq!(
                    retried.run_at,
                    now + Duration::seconds(retry_delay_seconds(4, TEST_BASE_RETRY_DELAY_SECONDS))
                );
                assert_eq!(retried.updated_at, now);
                assert_eq!(retried.last_error, Some("boom".to_string()));
            }
            TaskFailureOutcome::DeadLetter(_) => panic!("expected a retry outcome"),
        }
    }

    #[test]
    fn it_should_grow_the_retry_delay_with_each_successive_failure() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();

        let mut task = task_with_retries(0);
        let mut previous_delay = None;
        for _ in 0..4 {
            match task
                .clone()
                .fail("boom", now, TEST_BASE_RETRY_DELAY_SECONDS)
            {
                TaskFailureOutcome::Retry(retried) => {
                    let delay = retried.run_at - now;
                    if let Some(previous_delay) = previous_delay {
                        assert!(delay > previous_delay);
                    }
                    previous_delay = Some(delay);
                    task = retried;
                }
                TaskFailureOutcome::DeadLetter(_) => panic!("expected a retry outcome"),
            }
        }
    }

    #[test]
    fn it_should_use_the_base_delay_passed_in_when_retrying() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();

        let outcome = task_with_retries(0).fail("boom", now, 10);

        match outcome {
            TaskFailureOutcome::Retry(retried) => {
                assert_eq!(
                    retried.run_at,
                    now + Duration::seconds(retry_delay_seconds(1, 10))
                );
                assert_ne!(
                    retried.run_at,
                    now + Duration::seconds(retry_delay_seconds(1, TEST_BASE_RETRY_DELAY_SECONDS))
                );
            }
            TaskFailureOutcome::DeadLetter(_) => panic!("expected a retry outcome"),
        }
    }

    #[test]
    fn it_should_dead_letter_when_the_fifth_attempt_fails() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();

        let outcome = task_with_retries(4).fail("boom", now, TEST_BASE_RETRY_DELAY_SECONDS);

        match outcome {
            TaskFailureOutcome::DeadLetter(dead) => {
                assert_eq!(dead.original_task_id, 1);
                assert_eq!(dead.task_type, "reconcile_playlist");
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
