## Why

Playlists can be created and listed, but nothing keeps their video contents up to date after creation, and there is no infrastructure to run work asynchronously, on a schedule, or in reaction to things that happen elsewhere in the system. This change adds that infrastructure and uses it to implement the first real use case: syncing a playlist's videos from YouTube once when it's created, then automatically every hour after that, restart-safely.

## What Changes

- Add a persisted, asynchronous **domain event** mechanism: publishing an event durably records it; a background consumer polls for pending events and dispatches each one to every subscriber registered for its type, independently of the request that published it (so a slow/failing subscriber never affects the caller). Multiple subscribers per event type are supported; failures are tracked with a single retry counter per event and give up after a fixed number of attempts, on the assumption that subscribers are idempotent.
- Add a persisted, restart-safe **task scheduling** mechanism, structurally separate from domain events: a task is scheduled to run at (or after) a specific time, executed by a background task executor, and retried up to a fixed number of attempts before being dropped and logged. Tasks left in a `running` state after an unclean shutdown are recovered on startup and treated as a failed attempt.
- Add a new `subscribers/` top-level module: thin adapters that decode a domain event's payload and call exactly one domain service method, mirroring how `http/` and `cli/` translate their own triggers into domain calls.
- Add a new `tasks/` top-level module: thin adapters that decode a task's payload and call exactly one domain service method, following the same pattern.
- Add a `videos` table and a `sync_playlist` operation on the playlist domain service: fetches the current members of a YouTube playlist, upserts them into `videos` with status `PENDING`, deletes rows for videos no longer present in the source playlist (logging each deletion), and publishes one domain event per newly added video.
- `create_playlist` now publishes a `PlaylistCreated` domain event; a new subscriber reacts to it by calling `sync_playlist` for the new playlist.
- `delete_playlist` now publishes a `PlaylistDeleted` domain event. No subscriber reacts to it yet in this change.
- `sync_playlist` schedules its own recurring re-run as a task, at a configurable interval (`YARRTUBE_SYNC_INTERVAL_SECONDS`, default 3600), making the hourly sync cycle self-perpetuating without depending on a domain event firing on every run.
- Daemon startup now also recovers stuck tasks and starts the domain event consumer and task executor background loops, alongside the existing heartbeat/HTTP server startup.

Out of scope for this change: downloading videos. `videos.status` exists and is set to `PENDING`, but nothing consumes that status yet — that's a future change once a download task type is introduced.

## Capabilities

### New Capabilities
- `domain-events`: persisted, asynchronous publishing and multi-subscriber consumption of domain events, independent of any specific event type.
- `task-scheduling`: persisted, restart-safe scheduling and execution of time-based tasks, independent of any specific task type.
- `playlist-sync`: fetching a playlist's current videos from YouTube and keeping the `videos` table in sync with it, on creation and on a recurring interval.

### Modified Capabilities
- `playlist-crud`: creating a playlist now publishes a `PlaylistCreated` domain event; deleting a playlist now publishes a `PlaylistDeleted` domain event.
- `daemon`: startup now also recovers tasks left in a `running` state from a prior run, and starts the domain event consumer and task executor as long-running background loops for the life of the process.

## Impact

- New SQLite tables: `events`, `tasks`, `videos`.
- New modules: `domain/event.rs` (or `domain/events/`), `domain/task/`, `domain/video/`, `subscribers/`, `tasks/`.
- `infrastructure/shared/youtube_api.rs`'s plain functions are promoted to a trait-backed port (`infrastructure/repositories/`) so `sync_playlist` can depend on it via dependency injection and fake it in tests, consistent with how other external lookups are structured.
- `domain/playlist/service.rs` gains `sync_playlist` and publishes events from `create_playlist`/`delete_playlist`.
- `serve.rs` composition root wires up the new background loops (event consumer, task executor) and startup recovery, alongside the existing heartbeat loop.
- New configuration: `YARRTUBE_SYNC_INTERVAL_SECONDS` (default `3600`).
