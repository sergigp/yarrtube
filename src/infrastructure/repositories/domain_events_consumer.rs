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
    use crate::infrastructure::shared::domain_events::event_repository::FakeEventRepository;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    struct FakeSubscriber {
        calls: Arc<Mutex<Vec<String>>>,
        name: &'static str,
        fails: bool,
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

    fn clock() -> Arc<dyn Clock> {
        Arc::new(FixedClock(now()))
    }

    #[test]
    fn it_should_invoke_the_single_registered_subscriber_and_delete_the_event() {
        let repository = Arc::new(FakeEventRepository::seeded("playlist_created"));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let subscriber = Arc::new(FakeSubscriber {
            calls: calls.clone(),
            name: "sub1",
            fails: false,
        });
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert("playlist_created".to_string(), vec![subscriber]);
        let consumer = DomainEventsConsumer::new(repository.clone(), subscribers, clock());

        consumer.poll_once().unwrap();

        assert_eq!(*calls.lock().unwrap(), vec!["sub1"]);
        assert_eq!(*repository.deleted.lock().unwrap(), vec![1]);
    }

    #[test]
    fn it_should_invoke_every_registered_subscriber() {
        let repository = Arc::new(FakeEventRepository::seeded("playlist_created"));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let sub1 = Arc::new(FakeSubscriber {
            calls: calls.clone(),
            name: "sub1",
            fails: false,
        });
        let sub2 = Arc::new(FakeSubscriber {
            calls: calls.clone(),
            name: "sub2",
            fails: false,
        });
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert("playlist_created".to_string(), vec![sub1, sub2]);
        let consumer = DomainEventsConsumer::new(repository, subscribers, clock());

        consumer.poll_once().unwrap();

        assert_eq!(*calls.lock().unwrap(), vec!["sub1", "sub2"]);
    }

    #[test]
    fn it_should_retry_the_whole_event_when_one_subscriber_fails() {
        let repository = Arc::new(FakeEventRepository::seeded("playlist_created"));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let failing = Arc::new(FakeSubscriber {
            calls: calls.clone(),
            name: "failing",
            fails: true,
        });
        let succeeding = Arc::new(FakeSubscriber {
            calls: calls.clone(),
            name: "succeeding",
            fails: false,
        });
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert("playlist_created".to_string(), vec![failing, succeeding]);
        let consumer = DomainEventsConsumer::new(repository.clone(), subscribers, clock());

        consumer.poll_once().unwrap();

        assert_eq!(*calls.lock().unwrap(), vec!["failing", "succeeding"]);
        let updated = repository.updated.lock().unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].retries, 1);
        assert!(repository.deleted.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_dead_letter_an_event_that_fails_on_the_fifth_attempt() {
        let repository = Arc::new(FakeEventRepository::seeded_with_retries(
            "playlist_created",
            4,
        ));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let failing = Arc::new(FakeSubscriber {
            calls: calls.clone(),
            name: "failing",
            fails: true,
        });
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert("playlist_created".to_string(), vec![failing]);
        let consumer = DomainEventsConsumer::new(repository.clone(), subscribers, clock());

        consumer.poll_once().unwrap();

        let dead_lettered = repository.dead_lettered.lock().unwrap();
        assert_eq!(dead_lettered.len(), 1);
        assert_eq!(dead_lettered[0].retries, 5);
    }

    #[test]
    fn it_should_mark_an_event_with_no_registered_subscribers_as_done() {
        let repository = Arc::new(FakeEventRepository::seeded("unregistered_type"));
        let consumer = DomainEventsConsumer::new(repository.clone(), HashMap::new(), clock());

        consumer.poll_once().unwrap();

        assert_eq!(*repository.deleted.lock().unwrap(), vec![1]);
    }
}
