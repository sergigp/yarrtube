use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_event_repository::EventRepository;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub type SubscriberRegistry = HashMap<String, Vec<Arc<dyn EventSubscriber>>>;

/// Domain-agnostic: polls `EventRepository` for pending events and dispatches
/// each to every subscriber registered for its type. Knows nothing about any
/// specific event or subscriber — the mapping is supplied by the app.
pub struct DomainEventsConsumer {
    repository: Arc<dyn EventRepository>,
    subscribers: SubscriberRegistry,
}

impl DomainEventsConsumer {
    pub fn new(repository: Arc<dyn EventRepository>, subscribers: SubscriberRegistry) -> Self {
        Self {
            repository,
            subscribers,
        }
    }

    pub fn poll_once(&self) -> anyhow::Result<()> {
        for event in self.repository.list_eligible()? {
            let mut failure: Option<String> = None;

            match self.subscribers.get(&event.event_type) {
                Some(subscribers) => {
                    println!(
                        "[events] dispatching event {} ({}) to {} subscriber(s)",
                        event.id,
                        event.event_type,
                        subscribers.len()
                    );
                    for subscriber in subscribers {
                        if let Err(e) = subscriber.handle(&event.payload) {
                            failure = Some(e.to_string());
                        }
                    }
                }
                None => println!(
                    "[events] event {} ({}) has no registered subscribers, marking done",
                    event.id, event.event_type
                ),
            }

            match failure {
                Some(error) => self.repository.mark_failed_or_retry(event.id, &error)?,
                None => self.repository.mark_done(event.id)?,
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
                Ok(Err(e)) => eprintln!("[domain-events] poll failed: {e}"),
                Err(e) => eprintln!("[domain-events] poll task panicked: {e}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::DomainEvent;
    use crate::infrastructure::repositories::sqlite_event_repository::PersistedEvent;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeEventRepository {
        events: Mutex<Vec<PersistedEvent>>,
        done: Mutex<Vec<i64>>,
        retried: Mutex<Vec<i64>>,
    }

    impl FakeEventRepository {
        fn seeded(event_type: &str) -> Self {
            let repo = Self::default();
            repo.events.lock().unwrap().push(PersistedEvent {
                id: 1,
                event_type: event_type.to_string(),
                payload: DomainEvent::PlaylistCreated {
                    playlist_id: "PL1".to_string(),
                }
                .payload()
                .to_string(),
            });
            repo
        }
    }

    impl EventRepository for FakeEventRepository {
        fn insert_pending(&self, _event: &DomainEvent) -> anyhow::Result<()> {
            unimplemented!("not exercised by the consumer")
        }

        fn list_eligible(&self) -> anyhow::Result<Vec<PersistedEvent>> {
            Ok(self.events.lock().unwrap().clone())
        }

        fn mark_done(&self, id: i64) -> anyhow::Result<()> {
            self.done.lock().unwrap().push(id);
            Ok(())
        }

        fn mark_failed_or_retry(&self, id: i64, _error: &str) -> anyhow::Result<()> {
            self.retried.lock().unwrap().push(id);
            Ok(())
        }
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

    #[test]
    fn it_should_invoke_the_single_registered_subscriber_and_mark_the_event_done() {
        let repository = Arc::new(FakeEventRepository::seeded("playlist_created"));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let subscriber = Arc::new(FakeSubscriber {
            calls: calls.clone(),
            name: "sub1",
            fails: false,
        });
        let mut subscribers: SubscriberRegistry = HashMap::new();
        subscribers.insert("playlist_created".to_string(), vec![subscriber]);
        let consumer = DomainEventsConsumer::new(repository.clone(), subscribers);

        consumer.poll_once().unwrap();

        assert_eq!(*calls.lock().unwrap(), vec!["sub1"]);
        assert_eq!(*repository.done.lock().unwrap(), vec![1]);
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
        let consumer = DomainEventsConsumer::new(repository, subscribers);

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
        let consumer = DomainEventsConsumer::new(repository.clone(), subscribers);

        consumer.poll_once().unwrap();

        assert_eq!(*calls.lock().unwrap(), vec!["failing", "succeeding"]);
        assert_eq!(*repository.retried.lock().unwrap(), vec![1]);
        assert!(repository.done.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_mark_an_event_with_no_registered_subscribers_as_done() {
        let repository = Arc::new(FakeEventRepository::seeded("unregistered_type"));
        let consumer = DomainEventsConsumer::new(repository.clone(), HashMap::new());

        consumer.poll_once().unwrap();

        assert_eq!(*repository.done.lock().unwrap(), vec![1]);
    }
}
