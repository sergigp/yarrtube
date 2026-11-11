# Proposal

## Why

A failing domain event is retried on every consumer poll (5 s by default) and dead-lettered after 5 attempts, about 25 s after its first failure. On 28 Oct Plex was unreachable for ~2.5 minutes and 4 events (`video_downloaded` ×2, `video_subtitles_fetched` ×2) were dead-lettered with only the outermost error ("error sending request for url (…/library/sections)"). Tasks already survive outages like this with an exponential backoff; events should too.

## What Changes

- A failed event is no longer eligible again on the next poll: it is retried after `base * 3^retries` seconds (the same rule tasks use), so with the default 150 s base its 5 attempts span about 5 hours instead of ~25 s. The attempt limit (5) and dead-letter behaviour are unchanged.
- Events share the task retry base delay setting (`YARRTUBE_RETRY_BASE_DELAY_SECONDS`, default 150 s) rather than getting a setting of their own: both cover the same failure class (a dependency of ours being briefly unavailable), one knob is easier to reason about, and the e2e suites' `0`/`10` overrides keep both fast with no new variable.
- The consumer still processes eligible events in id order. An event that is backing off can be overtaken by later events, including later events of the same aggregate. This already happens today within a single poll (a failed event stays pending while the later events in that poll succeed); the backoff only widens the window from seconds to hours. It is acceptable because every subscriber acts on current state rather than on the event's ordering: reconcilers and file deletions no-op on a source that no longer exists, sync subscribers schedule a task that re-reads the video, and Plex scans/label syncs are idempotent.
- An event waiting out its retry delay is still a pending event everywhere except the consumer's dispatch: the move guard (pending source deletions reserve their path, unhandled downloads keep a show unsettled) and the work-in-progress flag of the channel and playlist lists keep seeing it. Otherwise a deleted source whose deletion event is backing off would stop reserving its folder, a show could be moved there, and the event's retry would delete it.
- An event's stored error (`last_error` on the event and on its dead letter) is the failing subscriber's full cause chain (`error_report::cause_chain`), as for tasks, instead of only its outermost message.
- Migration adds `run_at` to `events`; existing rows get `run_at = created_at` so they stay eligible immediately; newly published events are eligible at publish time.

Out of scope: moving the initial reconcile out of the `ChannelCreated`/`PlaylistCreated` subscribers, making subscribers best-effort, anything about the task executor.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `domain-events`: "Retry Then Give Up" gains a growing delay between attempts and ordering among eligible events; the stored error records the full cause chain.

## Impact

- `migrations/0029_event_run_at.sql` + registration in `src/infrastructure/shared/sqlite_migrations.rs`.
- `src/domain/event/scheduled_event.rs`: `run_at` field; `fail` takes the base delay and sets `run_at`, reusing the task delay rule.
- `src/domain/task/scheduled_task.rs`: the delay rule moves to a shared home both entities use.
- `src/infrastructure/shared/domain_events/event_repository.rs`, `event_publisher.rs`: read/write `run_at`; the repository reads a `Clock` and `list_eligible()` filters `run_at <= now`; new `list_pending()` returns every pending event.
- `src/domain/services/show_mover.rs`, `channel_view_searcher.rs`, `playlist_view_searcher.rs`: read `list_pending()` instead of `list_eligible()`.
- `src/infrastructure/repositories/domain_events_consumer.rs`: takes the base delay, records `cause_chain`.
- `src/serve.rs`: passes `retry_base_delay_seconds()` to the consumer.
- `doc/INSTALLATION.md`: `YARRTUBE_RETRY_BASE_DELAY_SECONDS` row also covers events.
