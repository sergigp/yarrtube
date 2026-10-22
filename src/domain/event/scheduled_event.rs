use crate::domain::channel::ChannelDeleted;
use crate::domain::playlist::PlaylistDeleted;
use crate::domain::video::VideoDownloaded;
use chrono::{DateTime, Utc};
use serde::Deserialize;

const MAX_ATTEMPTS: i64 = 5;

#[derive(Deserialize)]
struct VideoDownloadedPayload {
    output_dir: String,
}

#[derive(Deserialize)]
struct SourceDeletedPayload {
    path: String,
}

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
    /// The show directory a `VideoDownloaded` event's subscriber writes into
    /// once handled; `None` for any other event type.
    pub fn output_dir(&self) -> Option<String> {
        (self.event_type == VideoDownloaded::EVENT_TYPE)
            .then(|| serde_json::from_str::<VideoDownloadedPayload>(&self.payload).ok())
            .flatten()
            .map(|payload| payload.output_dir)
    }

    /// The source path a `PlaylistDeleted` / `ChannelDeleted` event's
    /// subscriber schedules for recursive deletion once handled; `None` for
    /// any other event type.
    pub fn deleted_path(&self) -> Option<String> {
        [PlaylistDeleted::EVENT_TYPE, ChannelDeleted::EVENT_TYPE]
            .contains(&self.event_type.as_str())
            .then(|| serde_json::from_str::<SourceDeletedPayload>(&self.payload).ok())
            .flatten()
            .map(|payload| payload.path)
    }

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
    use crate::domain::event::DomainEvent;
    use crate::domain::video::VideoSubtitlesFetched;

    fn event_of(event: DomainEvent) -> ScheduledEvent {
        ScheduledEvent {
            event_type: event.event_type().to_string(),
            payload: event.payload().to_string(),
            ..event_with_retries(0)
        }
    }

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
    fn it_should_expose_the_output_dir_of_a_video_downloaded_event() {
        let event = event_of(DomainEvent::VideoDownloaded(VideoDownloaded {
            video_id: "rec1".to_string(),
            output_dir: "/videos/music".to_string(),
            folder: "Season 1".to_string(),
        }));

        assert_eq!(event.output_dir(), Some("/videos/music".to_string()));
    }

    #[test]
    fn it_should_expose_no_output_dir_for_other_events() {
        let event = event_of(DomainEvent::VideoSubtitlesFetched(VideoSubtitlesFetched {
            video_id: "rec1".to_string(),
            output_dir: "/videos/music".to_string(),
        }));

        assert_eq!(event.output_dir(), None);
    }

    #[test]
    fn it_should_expose_no_output_dir_for_an_undecodable_payload() {
        let event = ScheduledEvent {
            event_type: VideoDownloaded::EVENT_TYPE.to_string(),
            ..event_with_retries(0)
        };

        assert_eq!(event.output_dir(), None);
    }

    #[test]
    fn it_should_expose_the_deleted_path_of_a_source_deleted_event() {
        let events = [
            event_of(DomainEvent::PlaylistDeleted(PlaylistDeleted {
                playlist_id: "PL1".to_string(),
                name: "Chill".to_string(),
                path: "music/chill".to_string(),
            })),
            event_of(DomainEvent::ChannelDeleted(ChannelDeleted {
                channel_id: "@somechannel".to_string(),
                name: "Some Channel".to_string(),
                path: "music/chill".to_string(),
            })),
        ];

        let deleted_paths: Vec<Option<String>> =
            events.iter().map(ScheduledEvent::deleted_path).collect();

        assert_eq!(deleted_paths, vec![Some("music/chill".to_string()); 2]);
    }

    #[test]
    fn it_should_expose_no_deleted_path_for_other_events() {
        let event = event_of(DomainEvent::VideoDownloaded(VideoDownloaded {
            video_id: "rec1".to_string(),
            output_dir: "/videos/music".to_string(),
            folder: "Season 1".to_string(),
        }));

        assert_eq!(event.deleted_path(), None);
    }

    #[test]
    fn it_should_expose_no_deleted_path_for_an_undecodable_payload() {
        let event = ScheduledEvent {
            event_type: PlaylistDeleted::EVENT_TYPE.to_string(),
            ..event_with_retries(0)
        };

        assert_eq!(event.deleted_path(), None);
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
