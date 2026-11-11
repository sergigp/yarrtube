# Design

## Context

See proposal.md. Tasks already back off: `ScheduledTask::fail(error, now, base)` sets `run_at = now + retry_delay_seconds(retries, base)`, and `SqliteTaskRepository` (built with a `Clock`) filters `run_at <= clock.now()` in `list_eligible()`. Events have none of this.

## Decisions

- **Delay rule moves to `domain/shared/retry_delay.rs`** so tasks and events share one rule without the event aggregate importing the task one. Each entity keeps its own `MAX_ATTEMPTS = 5`.
- **Shared base delay.** The consumer takes the same `retry_base_delay_seconds()` `TaskExecutor` gets (reasons in proposal.md).
- **`SqliteEventRepository::new(db, clock)`, `list_eligible()` keeps its signature**, mirroring `SqliteTaskRepository`. Ordering stays `ORDER BY id ASC` (not `run_at, id` like tasks): publication order among eligible events.
- **`list_eligible()` is for dispatch only; everything else reads `list_pending()`** (every pending row, id order, no `run_at` filter), the same split tasks have between `list_eligible()` and `list_non_completed()`. `ShowMover` (`ensure_settled`, `pending_deletion_paths`) and both view searchers previously read `list_eligible()` as "all pending"; with backoff that would hide a backing-off deletion event, unreserve its path, and let its retry delete a show moved there.
- **Migration**: `ALTER TABLE events ADD COLUMN run_at TEXT NOT NULL DEFAULT ''` then `UPDATE events SET run_at = created_at` (SQLite can't add a `NOT NULL` column without a constant default). Rollback: an older binary refuses a database migrated past its last migration (`rusqlite_migration`'s `DatabaseTooFarAhead`), so rolling back means restoring a pre-upgrade database backup.
- **Cause chain**: the consumer keeps the `anyhow::Error` from the failing subscriber and renders `error_report::cause_chain(&e)`. Still the last failing subscriber's error when several fail (unchanged; out of scope).
- **Test churn**: ~185 `SqliteEventRepository::new(..)` sites gain `Arc::new(FixedClock(now()))`, and the ~10 `pending_event` helpers / `ScheduledEvent { .. }` literals gain `run_at`. Mechanical, done in the walking-skeleton task.

## Files

- `migrations/0029_event_run_at.sql`: adds `events.run_at`, backfills from `created_at`.
- `src/infrastructure/shared/sqlite_migrations.rs`: registers 0029.
- `src/domain/shared/retry_delay.rs` (new): `retry_delay_seconds`, shared by tasks and events.
- `src/domain/shared/mod.rs`: exports it.
- `src/domain/task/scheduled_task.rs`: uses the shared rule (constant and fn removed here).
- `src/infrastructure/repositories/sqlite_task_repository.rs`: test import path of `retry_delay_seconds`.
- `src/domain/event/scheduled_event.rs`: `run_at` field; `fail` takes `base_retry_delay_seconds` and sets `run_at`.
- `src/infrastructure/shared/domain_events/event_publisher.rs`: inserts `run_at = now`.
- `src/infrastructure/shared/domain_events/event_repository.rs`: holds a `Clock`; maps/updates `run_at` (one `timestamp(row, idx)` helper parses every timestamp column); `list_eligible` filters `run_at <= now`; `list_pending` returns every pending row.
- `src/domain/services/show_mover.rs`: `ensure_settled` and `pending_deletion_paths` read `list_pending()`.
- `src/domain/services/channel_view_searcher.rs`, `playlist_view_searcher.rs`: `search_all` reads `list_pending()`.
- `src/application/http/playlists/mod.rs`, `channels/mod.rs`: acceptance tests for the move guard and work-in-progress flag with a backing-off event.
- `src/infrastructure/infrastructure_container.rs`: passes the clock to `SqliteEventRepository::new`.
- `src/infrastructure/repositories/domain_events_consumer.rs`: takes `base_retry_delay_seconds`; records `cause_chain`.
- `src/serve.rs`: passes `retry_base_delay_seconds()` to `DomainEventsConsumer::new`.
- Every test module building `SqliteEventRepository` / `ScheduledEvent`: new constructor arg / field.
- `doc/INSTALLATION.md`: `YARRTUBE_RETRY_BASE_DELAY_SECONDS` row covers events too.

## Types & Signatures

```rust
// src/domain/shared/retry_delay.rs
pub fn retry_delay_seconds(retries: i64, base_delay_seconds: i64) -> i64; // base * 3^retries

// src/domain/event/scheduled_event.rs
pub struct ScheduledEvent {
    pub id: i64,
    pub event_type: String,
    pub payload: String,
    pub retries: i64,
    pub run_at: DateTime<Utc>,          // new
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_error: Option<String>,
}
impl ScheduledEvent {
    pub fn fail(
        self,
        error: impl Into<String>,
        now: DateTime<Utc>,
        base_retry_delay_seconds: i64,  // new
    ) -> EventFailureOutcome;           // Retry: run_at = now + retry_delay_seconds(retries, base)
}

// src/infrastructure/shared/domain_events/event_repository.rs
impl SqliteEventRepository {
    pub fn new(db: Database, clock: Arc<dyn Clock>) -> Self;
}
pub trait EventRepository: Send + Sync {
    /// Pending events whose `run_at` has passed, in id order. Dispatch only.
    fn list_eligible(&self) -> anyhow::Result<Vec<ScheduledEvent>>; // unchanged signature
    /// Every pending event, backing off or not, in id order.
    fn list_pending(&self) -> anyhow::Result<Vec<ScheduledEvent>>;  // new
    fn update(&self, event: &ScheduledEvent) -> anyhow::Result<()>; // now also writes run_at
    // delete, dead_letter unchanged
}

// src/infrastructure/repositories/domain_events_consumer.rs
impl DomainEventsConsumer {
    pub fn new(
        repository: Arc<dyn EventRepository>,
        subscribers: SubscriberRegistry,
        clock: Arc<dyn Clock>,
        base_retry_delay_seconds: i64,  // new
    ) -> Self;
}
```

## Call Stack

Poll with a failing subscriber:
- `DomainEventsConsumer::poll_once()`
  - `repository.list_eligible()` → `SELECT … WHERE status = 'pending' AND run_at <= ?1 ORDER BY id ASC` with `clock.now()`
  - per event: `dispatch(subscriber, &event.payload)` → `Err(e)` kept as `anyhow::Error`
  - `event.fail(error_report::cause_chain(&e), clock.now(), base_retry_delay_seconds)`
    - `Retry(retried)` → `repository.update(&retried)` (retries, run_at, updated_at, last_error)
    - `DeadLetter(dead)` → `repository.dead_letter(&dead)`

Publish:
- `SqliteEventPublisher::publish(event)` → `INSERT … run_at = now`

Move a show / list channels or playlists:
- `ShowMover::ensure_settled` → `event_repository.list_pending()` → `has_work_in_progress(.., &events, ..)`
- `ShowMover::pending_deletion_paths` → `event_repository.list_pending()` → `ScheduledEvent::deleted_path`
- `ChannelViewSearcher::search_all` / `PlaylistViewSearcher::search_all` → `event_repository.list_pending()` → work-in-progress flag

## Test Plan

Behaviour tests (through `DomainEventsConsumer::poll_once`, real SQLite, `FixedClock`, asserting `list_eligible()` rows and `list_dead_lettered()`):

1. `it_should_reschedule_a_failed_event_after_the_retry_delay` — one failure with base 150: stored row has `retries: 1`, `run_at: now + 450 s`, `last_error` set; nothing dead-lettered. (Asserted with a repository whose clock is past `run_at`, since `list_eligible` hides it otherwise.)
2. `it_should_not_dispatch_an_event_before_its_retry_delay` — event updated to `retries: 1, run_at: now + 450 s`, consumer clock at `now + 449 s`: subscriber not called, row unchanged.
3. `it_should_dispatch_an_event_once_its_retry_delay_has_passed` — same event, consumer clock at `now + 450 s`: subscriber called, event deleted.
4. `it_should_grow_the_retry_delay_with_each_failure` — event at `retries: 3` fails: `retries: 4`, `run_at: now + 150 * 81 s`.
5. `it_should_dispatch_a_later_event_while_an_earlier_one_backs_off` — event 1 backing off, event 2 eligible: only event 2's subscriber called, event 2 deleted, event 1 unchanged.
6. `it_should_dispatch_eligible_events_in_publication_order` — two eligible events: subscriber calls in id order.
7. `it_should_record_the_full_cause_chain_as_the_last_error` — subscriber fails with a context-wrapped error: stored `last_error` is `"outer: root"`.
8. `it_should_dead_letter_an_event_with_the_full_cause_chain` — same failure at `retries: 4`: dead letter's `last_error` is `"outer: root"`.

Existing consumer tests (`it_should_retry_the_whole_event_when_one_subscriber_fails`, the panic ones) get the new `run_at` in their expected rows.

9. `it_should_fail_to_move_into_the_folder_of_a_source_whose_deletion_is_backing_off` (`http/playlists`) — a `PlaylistDeleted` event updated to `run_at` in the future: moving a show into its path is rejected with the pending-deletion error; the show and its folder are unchanged.
10. `it_should_fail_to_move_if_a_downloaded_video_event_is_backing_off` (`http/playlists`) — a `VideoDownloaded` event for the show backing off: the move is rejected as work in progress.
11. `it_should_list_a_playlist_with_a_backing_off_downloaded_video_event_as_work_in_progress` (`http/playlists`).
12. `it_should_list_a_channel_with_a_backing_off_downloaded_video_event_as_work_in_progress` (`http/channels`).

Infrastructure tests (`event_repository.rs`, real SQLite):

13. `it_should_list_a_newly_published_event_as_eligible` — published at `now`, repo clock `now`: listed with `run_at: now`.
14. `it_should_not_list_an_event_whose_run_at_is_in_the_future` — updated with `run_at: now + 1 s`: not listed; listed by a repo whose clock is `now + 1 s`.
15. `it_should_keep_pre_migration_events_eligible` (in `sqlite_migrations.rs`, alongside the other migration tests, migrating to 0028 with `migrate_to`) — row inserted before 0029: after migrating, `run_at == created_at`.
16. `it_should_list_a_backing_off_event_as_pending` — updated with `run_at: now + 1 s`: hidden from `list_eligible()`, returned by `list_pending()`.
