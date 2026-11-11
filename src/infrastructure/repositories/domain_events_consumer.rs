use crate::domain::event::{EventFailureOutcome, ScheduledEvent};
use crate::domain::shared::elapsed_seconds;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::shared::domain_events::event_repository::EventRepository;
use crate::infrastructure::shared::error_report;
use crate::infrastructure::shared::panic_message::panic_message;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

pub type SubscriberRegistry = HashMap<String, Vec<Arc<dyn EventSubscriber>>>;

/// Domain-agnostic: polls `EventRepository` for pending events and dispatches
/// each to every subscriber registered for its type. Knows nothing about any
/// specific event or subscriber — the mapping is supplied by the app.
pub struct DomainEventsConsumer {
    repository: Arc<dyn EventRepository>,
    subscribers: SubscriberRegistry,
    clock: Arc<dyn Clock>,
    base_retry_delay_seconds: i64,
}

impl DomainEventsConsumer {
    pub fn new(
        repository: Arc<dyn EventRepository>,
        subscribers: SubscriberRegistry,
        clock: Arc<dyn Clock>,
        base_retry_delay_seconds: i64,
    ) -> Self {
        Self {
            repository,
            subscribers,
            clock,
            base_retry_delay_seconds,
        }
    }

    pub fn poll_once(&self) -> anyhow::Result<()> {
        self.repository
            .list_eligible()?
            .into_iter()
            .try_for_each(|event| self.consume(event))
    }

    /// Polls forever on `interval`, matching `heartbeat_loop`'s shape. Meant
    /// to be handed to `tokio::spawn` by the composition root.
    pub async fn run(self: Arc<Self>, interval: Duration) {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let consumer = self.clone();
            match tokio::task::spawn_blocking(move || consumer.poll_once()).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => error!(error = %e, "poll failed"),
                Err(e) => error!(error = %e, "poll task panicked"),
            }
        }
    }

    /// Dispatches `event` to every subscriber of its type, then deletes it,
    /// or records the failed attempt; one end line either way.
    fn consume(&self, event: ScheduledEvent) -> anyhow::Result<()> {
        let started = self.clock.now();
        let subscribers = self
            .subscribers
            .get(&event.event_type)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let failure = Self::dispatch_all(&event, subscribers);
        let duration_seconds = elapsed_seconds(started, self.clock.now());
        match failure {
            Some(error) => self.record_failure(event, error, subscribers.len(), duration_seconds),
            None => {
                self.repository.delete(event.id)?;
                info!(
                    event_id = event.id,
                    event_type = %event.event_type,
                    payload = %event.payload,
                    subscriber_count = subscribers.len(),
                    outcome = "done",
                    duration_seconds,
                    "event finished"
                );
                Ok(())
            }
        }
    }

    /// Hands `event` to each of `subscribers`, all of them even after one
    /// fails; the last failure's full cause chain, if any.
    fn dispatch_all(
        event: &ScheduledEvent,
        subscribers: &[Arc<dyn EventSubscriber>],
    ) -> Option<String> {
        if subscribers.is_empty() {
            warn!(
                event_id = event.id,
                event_type = %event.event_type,
                "event has no registered subscribers, marking done"
            );
            return None;
        }
        info!(
            event_id = event.id,
            event_type = %event.event_type,
            subscriber_count = subscribers.len(),
            "dispatching event"
        );
        // The full source chain, not just the outermost context, so the
        // stored error names the real cause.
        subscribers.iter().fold(None, |failure, subscriber| {
            Self::dispatch(subscriber.as_ref(), &event.payload)
                .err()
                .map(|e| error_report::cause_chain(&e))
                .or(failure)
        })
    }

    /// Retries `event` later, or dead-letters it once its attempts run out.
    fn record_failure(
        &self,
        event: ScheduledEvent,
        error: String,
        subscriber_count: usize,
        duration_seconds: f64,
    ) -> anyhow::Result<()> {
        let (event_type, payload) = (event.event_type.clone(), event.payload.clone());
        match event.fail(
            error.clone(),
            self.clock.now(),
            self.base_retry_delay_seconds,
        ) {
            EventFailureOutcome::Retry(retried) => {
                warn!(
                    event_id = retried.id,
                    event_type = %event_type,
                    payload = %payload,
                    subscriber_count,
                    retries = retried.retries,
                    run_at = %retried.run_at,
                    error,
                    duration_seconds,
                    "event failed, retrying"
                );
                self.repository.update(&retried)
            }
            EventFailureOutcome::DeadLetter(dead) => {
                error!(
                    event_id = dead.original_event_id,
                    event_type = %event_type,
                    payload = %payload,
                    subscriber_count,
                    retries = dead.retries,
                    error,
                    duration_seconds,
                    "event failed permanently"
                );
                self.repository.dead_letter(&dead)
            }
        }
    }

    /// A subscriber that panics counts as a failed invocation, like one that
    /// returns an error, so its event is retried or dead-lettered instead of
    /// the panic escaping the poll and leaving the event untouched.
    fn dispatch(subscriber: &dyn EventSubscriber, payload: &str) -> anyhow::Result<()> {
        panic::catch_unwind(AssertUnwindSafe(|| subscriber.handle(payload))).unwrap_or_else(
            |payload| {
                Err(anyhow::anyhow!(
                    "subscriber panicked: {}",
                    panic_message(payload.as_ref())
                ))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tasks::log_capture::captured_info_lines;
    use crate::domain::channel::ChannelCreated;
    use crate::domain::event::{DeadLetteredEvent, DomainEvent, ScheduledEvent};
    use crate::domain::playlist::PlaylistCreated;
    use crate::infrastructure::shared::domain_events::event_publisher::{
        EventPublisher, SqliteEventPublisher,
    };
    use crate::infrastructure::shared::domain_events::event_repository::SqliteEventRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Duration, Utc};
    use std::sync::Mutex;

    #[test]
    fn it_should_invoke_the_single_registered_subscriber_and_delete_the_event() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let subscriber = Arc::new(FakeSubscriber::succeeding("sub1", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![subscriber]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), vec!["sub1"]);
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            Vec::<ScheduledEvent>::new()
        );
        assert_eq!(
            event_repository.list_dead_lettered().unwrap(),
            Vec::<DeadLetteredEvent>::new()
        );
    }

    #[test]
    fn it_should_invoke_every_registered_subscriber() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let sub1 = Arc::new(FakeSubscriber::succeeding("sub1", calls.clone()));
        let sub2 = Arc::new(FakeSubscriber::succeeding("sub2", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![sub1, sub2]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), vec!["sub1", "sub2"]);
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            Vec::<ScheduledEvent>::new()
        );
    }

    #[test]
    fn it_should_retry_the_whole_event_when_one_subscriber_fails() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let failing = Arc::new(FakeSubscriber::failing("failing", calls.clone()));
        let succeeding = Arc::new(FakeSubscriber::succeeding("succeeding", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![failing, succeeding]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), vec!["failing", "succeeding"]);
        assert_eq!(
            repository_at(&db, now() + Duration::seconds(450))
                .list_eligible()
                .unwrap(),
            vec![ScheduledEvent {
                retries: 1,
                run_at: now() + Duration::seconds(450),
                last_error: Some("failing failed".to_string()),
                ..pending_event(0)
            }]
        );
        assert_eq!(
            event_repository.list_dead_lettered().unwrap(),
            Vec::<DeadLetteredEvent>::new()
        );
    }

    #[test]
    fn it_should_dead_letter_an_event_that_fails_on_the_fifth_attempt() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let failing = Arc::new(FakeSubscriber::failing("failing", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        event_repository.update(&pending_event(4)).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![failing]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), vec!["failing"]);
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            Vec::<ScheduledEvent>::new()
        );
        assert_eq!(
            event_repository.list_dead_lettered().unwrap(),
            vec![DeadLetteredEvent {
                original_event_id: 1,
                event_type: event().event_type().to_string(),
                payload: event().payload().to_string(),
                retries: 5,
                last_error: "failing failed".to_string(),
                created_at: now(),
                failed_at: now(),
            }]
        );
    }

    #[test]
    fn it_should_retry_an_event_whose_subscriber_panics() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let panicking = Arc::new(FakeSubscriber::panicking("panicking", calls.clone()));
        let succeeding = Arc::new(FakeSubscriber::succeeding("succeeding", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(
            event().event_type().to_string(),
            vec![panicking, succeeding],
        );
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), vec!["panicking", "succeeding"]);
        assert_eq!(
            repository_at(&db, now() + Duration::seconds(450))
                .list_eligible()
                .unwrap(),
            vec![ScheduledEvent {
                retries: 1,
                run_at: now() + Duration::seconds(450),
                last_error: Some("subscriber panicked: panicking panicked".to_string()),
                ..pending_event(0)
            }]
        );
        assert_eq!(
            event_repository.list_dead_lettered().unwrap(),
            Vec::<DeadLetteredEvent>::new()
        );
    }

    #[test]
    fn it_should_dead_letter_an_event_whose_subscriber_panics_on_the_fifth_attempt() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let panicking = Arc::new(FakeSubscriber::panicking("panicking", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        event_repository.update(&pending_event(4)).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![panicking]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), vec!["panicking"]);
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            Vec::<ScheduledEvent>::new()
        );
        assert_eq!(
            event_repository.list_dead_lettered().unwrap(),
            vec![DeadLetteredEvent {
                original_event_id: 1,
                event_type: event().event_type().to_string(),
                payload: event().payload().to_string(),
                retries: 5,
                last_error: "subscriber panicked: panicking panicked".to_string(),
                created_at: now(),
                failed_at: now(),
            }]
        );
    }

    #[test]
    fn it_should_process_the_other_pending_events_if_a_subscriber_panics() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let panicking = Arc::new(FakeSubscriber::panicking("panicking", calls.clone()));
        let succeeding = Arc::new(FakeSubscriber::succeeding("succeeding", calls.clone()));
        let other_event = DomainEvent::ChannelCreated(ChannelCreated {
            channel_id: "UC1".to_string(),
        });
        event_publisher.publish(&event()).unwrap();
        event_publisher.publish(&other_event).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![panicking]);
        subscribers.insert(other_event.event_type().to_string(), vec![succeeding]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), vec!["panicking", "succeeding"]);
        assert_eq!(
            repository_at(&db, now() + Duration::seconds(450))
                .list_eligible()
                .unwrap(),
            vec![ScheduledEvent {
                retries: 1,
                run_at: now() + Duration::seconds(450),
                last_error: Some("subscriber panicked: panicking panicked".to_string()),
                ..pending_event(0)
            }]
        );
    }

    #[test]
    fn it_should_mark_an_event_with_no_registered_subscribers_as_done() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        event_publisher.publish(&event()).unwrap();
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            HashMap::new(),
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            Vec::<ScheduledEvent>::new()
        );
        assert_eq!(
            event_repository.list_dead_lettered().unwrap(),
            Vec::<DeadLetteredEvent>::new()
        );
    }

    #[test]
    fn it_should_reschedule_a_failed_event_after_the_retry_delay() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let failing = Arc::new(FakeSubscriber::failing("failing", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![failing]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), vec!["failing"]);
        assert_eq!(
            repository_at(&db, now() + Duration::seconds(450))
                .list_eligible()
                .unwrap(),
            vec![ScheduledEvent {
                retries: 1,
                run_at: now() + Duration::seconds(450),
                last_error: Some("failing failed".to_string()),
                ..pending_event(0)
            }]
        );
        assert_eq!(
            event_repository.list_dead_lettered().unwrap(),
            Vec::<DeadLetteredEvent>::new()
        );
    }

    #[test]
    fn it_should_not_dispatch_an_event_before_its_retry_delay() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let poll_time = now() + Duration::seconds(449);
        let event_repository = Arc::new(repository_at(&db, poll_time));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let succeeding = Arc::new(FakeSubscriber::succeeding("succeeding", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        event_repository.update(&backing_off_event()).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![succeeding]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(poll_time)),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), Vec::<String>::new());
        assert_eq!(
            repository_at(&db, now() + Duration::seconds(450))
                .list_eligible()
                .unwrap(),
            vec![backing_off_event()]
        );
    }

    #[test]
    fn it_should_dispatch_an_event_once_its_retry_delay_has_passed() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let poll_time = now() + Duration::seconds(450);
        let event_repository = Arc::new(repository_at(&db, poll_time));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let succeeding = Arc::new(FakeSubscriber::succeeding("succeeding", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        event_repository.update(&backing_off_event()).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![succeeding]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(poll_time)),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), vec!["succeeding"]);
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            Vec::<ScheduledEvent>::new()
        );
    }

    #[test]
    fn it_should_grow_the_retry_delay_with_each_failure() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let failing = Arc::new(FakeSubscriber::failing("failing", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        event_repository.update(&pending_event(3)).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![failing]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(
            repository_at(&db, now() + Duration::seconds(150 * 81))
                .list_eligible()
                .unwrap(),
            vec![ScheduledEvent {
                retries: 4,
                run_at: now() + Duration::seconds(150 * 81),
                last_error: Some("failing failed".to_string()),
                ..pending_event(0)
            }]
        );
    }

    #[test]
    fn it_should_dispatch_a_later_event_while_an_earlier_one_backs_off() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let succeeding = Arc::new(FakeSubscriber::succeeding("succeeding", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        event_publisher.publish(&event()).unwrap();
        event_repository.update(&backing_off_event()).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![succeeding]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), vec!["succeeding"]);
        assert_eq!(
            repository_at(&db, now() + Duration::seconds(450))
                .list_eligible()
                .unwrap(),
            vec![backing_off_event()]
        );
    }

    #[test]
    fn it_should_dispatch_eligible_events_in_publication_order() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let channel = Arc::new(FakeSubscriber::succeeding("channel", calls.clone()));
        let playlist = Arc::new(FakeSubscriber::succeeding("playlist", calls.clone()));
        let channel_created = DomainEvent::ChannelCreated(ChannelCreated {
            channel_id: "@somechannel".to_string(),
        });
        event_publisher.publish(&channel_created).unwrap();
        event_publisher.publish(&event()).unwrap();
        event_publisher.publish(&channel_created).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(channel_created.event_type().to_string(), vec![channel]);
        subscribers.insert(event().event_type().to_string(), vec![playlist]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(
            *calls.lock().unwrap(),
            vec!["channel", "playlist", "channel"]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            Vec::<ScheduledEvent>::new()
        );
    }

    #[test]
    fn it_should_record_the_full_cause_chain_as_the_last_error() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let failing = Arc::new(FakeSubscriber::failing_with_cause("failing", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![failing]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(
            repository_at(&db, now() + Duration::seconds(450))
                .list_eligible()
                .unwrap(),
            vec![ScheduledEvent {
                last_error: Some("failing failed: Connection refused".to_string()),
                ..backing_off_event()
            }]
        );
    }

    #[test]
    fn it_should_dead_letter_an_event_with_the_full_cause_chain() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(repository_at(&db, now()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let failing = Arc::new(FakeSubscriber::failing_with_cause("failing", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        event_repository.update(&pending_event(4)).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![failing]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(
            event_repository.list_dead_lettered().unwrap(),
            vec![DeadLetteredEvent {
                original_event_id: 1,
                event_type: event().event_type().to_string(),
                payload: event().payload().to_string(),
                retries: 5,
                last_error: "failing failed: Connection refused".to_string(),
                created_at: now(),
                failed_at: now(),
            }]
        );
    }

    const TEST_BASE_RETRY_DELAY_SECONDS: i64 = 150;

    #[test]
    fn it_should_log_one_line_when_an_event_finishes() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        event_publisher.publish(&event()).unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(
            event().event_type().to_string(),
            vec![Arc::new(FakeSubscriber::succeeding("sub1", calls))],
        );
        let consumer = DomainEventsConsumer::new(
            Arc::new(repository_at(&db, now())),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let logs = captured_info_lines(|| consumer.poll_once().unwrap());

        assert_eq!(
            logs,
            vec![
                "INFO dispatching event event_id=1 event_type=playlist_created subscriber_count=1",
                "INFO event finished event_id=1 event_type=playlist_created payload={\"playlist_id\":\"PL1\"} subscriber_count=1 outcome=\"done\" duration_seconds=0.0",
            ]
        );
    }

    #[test]
    fn it_should_log_a_failed_event_with_its_fields() {
        let db = TestDatabase::new();
        let event_publisher = SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())));
        event_publisher.publish(&event()).unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(
            event().event_type().to_string(),
            vec![Arc::new(FakeSubscriber::failing("sub1", calls))],
        );
        let event_repository = Arc::new(repository_at(&db, now()));
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
            TEST_BASE_RETRY_DELAY_SECONDS,
        );

        let logs = captured_info_lines(|| consumer.poll_once().unwrap());

        let next_run_at = event_repository.list_pending().unwrap()[0].run_at;
        assert_eq!(
            logs,
            vec![
                "INFO dispatching event event_id=1 event_type=playlist_created subscriber_count=1"
                    .to_string(),
                format!(
                    "WARN event failed, retrying event_id=1 event_type=playlist_created payload={{\"playlist_id\":\"PL1\"}} subscriber_count=1 retries=1 run_at={next_run_at} error=\"sub1 failed\" duration_seconds=0.0"
                ),
            ]
        );
    }

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    /// The events as a repository whose clock reads `at` sees them.
    fn repository_at(db: &TestDatabase, at: DateTime<Utc>) -> SqliteEventRepository {
        SqliteEventRepository::new(db.database(), Arc::new(FixedClock(at)))
    }

    fn event() -> DomainEvent {
        DomainEvent::PlaylistCreated(PlaylistCreated {
            playlist_id: "PL1".to_string(),
        })
    }

    /// The row `publish(&event())` stores as id 1, with `retries`.
    fn pending_event(retries: i64) -> ScheduledEvent {
        ScheduledEvent {
            id: 1,
            event_type: event().event_type().to_string(),
            payload: event().payload().to_string(),
            retries,
            run_at: now(),
            created_at: now(),
            updated_at: now(),
            last_error: None,
        }
    }

    /// `pending_event(0)` after one failure: next attempt due 450 s later.
    fn backing_off_event() -> ScheduledEvent {
        ScheduledEvent {
            retries: 1,
            run_at: now() + Duration::seconds(450),
            last_error: Some("failing failed".to_string()),
            ..pending_event(0)
        }
    }

    struct FakeSubscriber {
        name: &'static str,
        outcome: FakeOutcome,
        calls: Arc<Mutex<Vec<String>>>,
    }

    enum FakeOutcome {
        Succeed,
        Fail,
        FailWithCause,
        Panic,
    }

    impl FakeSubscriber {
        fn succeeding(name: &'static str, calls: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name,
                outcome: FakeOutcome::Succeed,
                calls,
            }
        }

        fn failing(name: &'static str, calls: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name,
                outcome: FakeOutcome::Fail,
                calls,
            }
        }

        fn failing_with_cause(name: &'static str, calls: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name,
                outcome: FakeOutcome::FailWithCause,
                calls,
            }
        }

        fn panicking(name: &'static str, calls: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name,
                outcome: FakeOutcome::Panic,
                calls,
            }
        }
    }

    impl EventSubscriber for FakeSubscriber {
        fn handle(&self, _payload: &str) -> anyhow::Result<()> {
            self.calls.lock().unwrap().push(self.name.to_string());
            match self.outcome {
                FakeOutcome::Succeed => Ok(()),
                FakeOutcome::Fail => anyhow::bail!("{} failed", self.name),
                FakeOutcome::FailWithCause => {
                    Err(anyhow::anyhow!("Connection refused")
                        .context(format!("{} failed", self.name)))
                }
                FakeOutcome::Panic => panic!("{} panicked", self.name),
            }
        }
    }
}
