use crate::domain::event::EventFailureOutcome;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::shared::domain_events::event_repository::EventRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::HashMap;
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
}

impl DomainEventsConsumer {
    pub fn new(
        repository: Arc<dyn EventRepository>,
        subscribers: SubscriberRegistry,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repository,
            subscribers,
            clock,
        }
    }

    pub fn poll_once(&self) -> anyhow::Result<()> {
        for event in self.repository.list_eligible()? {
            let mut failure: Option<String> = None;

            match self.subscribers.get(&event.event_type) {
                Some(subscribers) => {
                    info!(
                        event_id = event.id,
                        event_type = %event.event_type,
                        subscriber_count = subscribers.len(),
                        "dispatching event"
                    );
                    for subscriber in subscribers {
                        if let Err(e) = subscriber.handle(&event.payload) {
                            failure = Some(e.to_string());
                        }
                    }
                }
                None => warn!(
                    event_id = event.id,
                    event_type = %event.event_type,
                    "event has no registered subscribers, marking done"
                ),
            }

            match failure {
                Some(error) => match event.fail(error.clone(), self.clock.now()) {
                    EventFailureOutcome::Retry(retried) => {
                        warn!(
                            event_id = retried.id,
                            retries = retried.retries,
                            error,
                            "event failed, retrying"
                        );
                        self.repository.update(&retried)?;
                    }
                    EventFailureOutcome::DeadLetter(dead) => {
                        error!(
                            event_id = dead.original_event_id,
                            retries = dead.retries,
                            error,
                            "event failed permanently"
                        );
                        self.repository.dead_letter(&dead)?;
                    }
                },
                None => {
                    self.repository.delete(event.id)?;
                    info!(event_id = event.id, "event done");
                }
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
            let consumer = self.clone();
            match tokio::task::spawn_blocking(move || consumer.poll_once()).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => error!(error = %e, "poll failed"),
                Err(e) => error!(error = %e, "poll task panicked"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::{DeadLetteredEvent, DomainEvent, ScheduledEvent};
    use crate::infrastructure::shared::domain_events::event_publisher::{
        EventPublisher, SqliteEventPublisher,
    };
    use crate::infrastructure::shared::domain_events::event_repository::SqliteEventRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

    #[test]
    fn it_should_invoke_the_single_registered_subscriber_and_delete_the_event() {
        let db = TestDatabase::new();
        let event_publisher =
            SqliteEventPublisher::new(db.shared_connection(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(SqliteEventRepository::new(db.shared_connection()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let subscriber = Arc::new(FakeSubscriber::succeeding("sub1", calls.clone()));
        event_publisher.publish(&event()).unwrap();
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert(event().event_type().to_string(), vec![subscriber]);
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            subscribers,
            Arc::new(FixedClock(now())),
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
        let event_publisher =
            SqliteEventPublisher::new(db.shared_connection(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(SqliteEventRepository::new(db.shared_connection()));
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
        let event_publisher =
            SqliteEventPublisher::new(db.shared_connection(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(SqliteEventRepository::new(db.shared_connection()));
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
        );

        let result = consumer.poll_once();

        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), vec!["failing", "succeeding"]);
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![ScheduledEvent {
                retries: 1,
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
        let event_publisher =
            SqliteEventPublisher::new(db.shared_connection(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(SqliteEventRepository::new(db.shared_connection()));
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
    fn it_should_mark_an_event_with_no_registered_subscribers_as_done() {
        let db = TestDatabase::new();
        let event_publisher =
            SqliteEventPublisher::new(db.shared_connection(), Arc::new(FixedClock(now())));
        let event_repository = Arc::new(SqliteEventRepository::new(db.shared_connection()));
        event_publisher.publish(&event()).unwrap();
        let consumer = DomainEventsConsumer::new(
            event_repository.clone(),
            HashMap::new(),
            Arc::new(FixedClock(now())),
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

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn event() -> DomainEvent {
        DomainEvent::PlaylistCreated {
            playlist_id: "PL1".to_string(),
        }
    }

    /// The row `publish(&event())` stores as id 1, with `retries`.
    fn pending_event(retries: i64) -> ScheduledEvent {
        ScheduledEvent {
            id: 1,
            event_type: event().event_type().to_string(),
            payload: event().payload().to_string(),
            retries,
            created_at: now(),
            updated_at: now(),
            last_error: None,
        }
    }

    struct FakeSubscriber {
        name: &'static str,
        fails: bool,
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl FakeSubscriber {
        fn succeeding(name: &'static str, calls: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name,
                fails: false,
                calls,
            }
        }

        fn failing(name: &'static str, calls: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name,
                fails: true,
                calls,
            }
        }
    }

    impl EventSubscriber for FakeSubscriber {
        fn handle(&self, _payload: &str) -> anyhow::Result<()> {
            self.calls.lock().unwrap().push(self.name.to_string());
            if self.fails {
                anyhow::bail!("{} failed", self.name);
            }
            Ok(())
        }
    }
}
