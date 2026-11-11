# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures): `migrations/0029_event_run_at.sql` adding `events.run_at TEXT NOT NULL DEFAULT ''` (no backfill yet) registered in `sqlite_migrations.rs`; `domain/shared/retry_delay.rs` with `retry_delay_seconds` moved from `scheduled_task.rs` (task code and `sqlite_task_repository.rs` tests import it from there); `ScheduledEvent.run_at`; `ScheduledEvent::fail(error, now, base_retry_delay_seconds)` setting `run_at: now` (stub); publisher inserting `run_at = now`; `SqliteEventRepository::new(db, clock)` mapping and updating `run_at`, `list_eligible` still unfiltered; `DomainEventsConsumer::new(.., base_retry_delay_seconds)` wired from `retry_base_delay_seconds()` in `serve.rs`; clock passed in `infrastructure_container.rs`; every test site updated mechanically (`SqliteEventRepository::new(.., Arc::new(FixedClock(now())))`, `run_at` in `pending_event` helpers / `ScheduledEvent` literals). Done when `cargo build` succeeds and `cargo test --locked` passes, with no new tests.

- [x] 1.2 Skeleton for the audit fix: add `EventRepository::list_pending()` with a stub body that returns `self.list_eligible()`, and switch `ShowMover::ensure_settled`, `ShowMover::pending_deletion_paths`, `ChannelViewSearcher::search_all` and `PlaylistViewSearcher::search_all` to it. Done when `cargo build` succeeds and `cargo test --locked` passes, with no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_reschedule_a_failed_event_after_the_retry_delay` — drives `fail` setting `run_at = now + retry_delay_seconds(retries, base)`; also update the `YARRTUBE_RETRY_BASE_DELAY_SECONDS` row in `doc/INSTALLATION.md` to say it applies to failed domain events too (e.g. a Plex scan after a download). Verify: test passes, `cargo test --locked` green.
- [x] 2.2 `it_should_not_dispatch_an_event_before_its_retry_delay` — drives `list_eligible` filtering `run_at <= clock.now()`. Verify: test passes, suite green.
- [x] 2.3 `it_should_dispatch_an_event_once_its_retry_delay_has_passed` — the boundary (`run_at == now`) is eligible. Verify: test passes, suite green.
- [x] 2.4 `it_should_grow_the_retry_delay_with_each_failure` — `retries: 3` → `retries: 4`, `run_at: now + 150 * 81 s`. Verify: test passes, suite green.
- [x] 2.5 `it_should_dispatch_a_later_event_while_an_earlier_one_backs_off` — a backing-off event does not hold back later ones. Verify: test passes, suite green.
- [x] 2.6 `it_should_dispatch_eligible_events_in_publication_order` — eligible events dispatched in id order. Verify: test passes, suite green.
- [x] 2.7 `it_should_record_the_full_cause_chain_as_the_last_error` — consumer keeps the `anyhow::Error` and stores `error_report::cause_chain`. Verify: test passes, suite green.
- [x] 2.8 `it_should_dead_letter_an_event_with_the_full_cause_chain` — dead letter's `last_error` is the full chain. Verify: test passes, suite green.
- [x] 2.9 `it_should_fail_to_move_into_the_folder_of_a_source_whose_deletion_is_backing_off` (`http/playlists`) — drives `list_pending()` returning every pending row regardless of `run_at`. Verify: test passes, suite green.
- [x] 2.10 `it_should_fail_to_move_if_a_downloaded_video_event_is_backing_off` (`http/playlists`) — `ensure_settled` sees backing-off downloads. Verify: test passes, suite green.
- [x] 2.11 `it_should_list_a_playlist_with_a_backing_off_downloaded_video_event_as_work_in_progress` (`http/playlists`). Verify: test passes, suite green.
- [x] 2.12 `it_should_list_a_channel_with_a_backing_off_downloaded_video_event_as_work_in_progress` (`http/channels`). Verify: test passes, suite green.

## 3. Infrastructure adapters (TDD)

SqliteEventRepository / SqliteEventPublisher (`event_repository.rs`):

- [x] 3.1 `it_should_list_a_newly_published_event_as_eligible` — published row has `run_at == created_at == now`. Verify: test passes, suite green.
- [x] 3.2 `it_should_not_list_an_event_whose_run_at_is_in_the_future` — hidden at `now`, listed by a repo whose clock is `now + 1 s`. Verify: test passes, suite green.
- [x] 3.4 `it_should_list_a_backing_off_event_as_pending` (`event_repository.rs`) — hidden from `list_eligible()`, returned by `list_pending()`. Verify: test passes, suite green.

Migrations (`sqlite_migrations.rs`):

- [x] 3.3 `it_should_keep_pre_migration_events_eligible` — drives the `UPDATE events SET run_at = created_at` backfill in 0029: an event inserted at migration 0028 has `run_at == created_at` after migrating to latest. Verify: test passes, suite green.

## 4. Verification

- [x] 4.1 Run `scripts/run-tests.sh --only rust` (fmt, clippy, tests); all pass. (No web changes, so `--only web` is not needed.)
- [x] 4.2 After 1.2–3.4, re-run `scripts/run-tests.sh --only rust` (fmt, clippy, tests); all pass.
