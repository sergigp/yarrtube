use chrono::{DateTime, Utc};

const MAX_ATTEMPTS: i64 = 5;

/// A persisted domain event's full lifecycle state. Unlike `ScheduledTask`,
/// there is no `running`/`start()` state: `DomainEventsConsumer` never marked
/// events running before dispatch, and there is no event crash-recovery
/// requirement today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledEvent {
    pub id: i64,
    pub event_type: String,
    pub payload: String,
    pub retries: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_error: Option<String>,
}

/// An event that exhausted its retry budget, shaped for the dead-letter table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadLetteredEvent {
    pub original_event_id: i64,
    pub event_type: String,
    pub payload: String,
    pub retries: i64,
    pub last_error: String,
    pub created_at: DateTime<Utc>,
    pub failed_at: DateTime<Utc>,
}

pub enum EventFailureOutcome {
    Retry(ScheduledEvent),
    DeadLetter(DeadLetteredEvent),
}

impl ScheduledEvent {
    pub fn fail(self, error: impl Into<String>, now: DateTime<Utc>) -> EventFailureOutcome {
        let error = error.into();
        let retries = self.retries + 1;

        if retries >= MAX_ATTEMPTS {
            EventFailureOutcome::DeadLetter(DeadLetteredEvent {
                original_event_id: self.id,
                event_type: self.event_type,
                payload: self.payload,
                retries,
                last_error: error,
                created_at: self.created_at,
                failed_at: now,
            })
        } else {
            EventFailureOutcome::Retry(Self {
                retries,
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

    fn event_with_retries(retries: i64) -> ScheduledEvent {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        ScheduledEvent {
            id: 1,
            event_type: "playlist_created".to_string(),
            payload: "{}".to_string(),
            retries,
            created_at: now,
            updated_at: now,
            last_error: None,
        }
    }

    #[test]
    fn it_should_retry_with_incremented_retries_when_below_the_attempt_limit() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();

        let outcome = event_with_retries(3).fail("boom", now);

        match outcome {
            EventFailureOutcome::Retry(retried) => {
                assert_eq!(retried.retries, 4);
                assert_eq!(retried.updated_at, now);
                assert_eq!(retried.last_error, Some("boom".to_string()));
            }
            EventFailureOutcome::DeadLetter(_) => panic!("expected a retry outcome"),
        }
    }

    #[test]
    fn it_should_dead_letter_when_the_fifth_attempt_fails() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();

        let outcome = event_with_retries(4).fail("boom", now);

        match outcome {
            EventFailureOutcome::DeadLetter(dead) => {
                assert_eq!(dead.original_event_id, 1);
                assert_eq!(dead.event_type, "playlist_created");
                assert_eq!(dead.retries, 5);
                assert_eq!(dead.last_error, "boom");
                assert_eq!(dead.failed_at, now);
            }
            EventFailureOutcome::Retry(_) => panic!("expected a dead-letter outcome"),
        }
    }
}
