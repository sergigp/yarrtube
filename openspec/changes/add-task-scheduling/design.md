## Context

See `proposal.md` - Why/What Changes for motivation and scope. Relevant current state this design builds on:

- Single Rust binary, single Docker container, no external services. `rusqlite::Connection` (blocking API) wrapped in a `Mutex`, driven from a tokio multi-threaded runtime.
- `serve.rs` already runs one long-lived background loop (`heartbeat_loop`, `tokio::spawn`'d, `tokio::time::interval`-based) — the shape new background loops in this change follow.
- Three-layer convention (`domain/`, `http/`, `infrastructure/`) plus `cli/` as a fourth adapter layer; ports are defined next to their implementation in `infrastructure/repositories/`; repositories are pure CQS with `anyhow::Result`; a service is scoped per aggregate and injected only with the ports its operations need.
- `infrastructure/shared/youtube_api.rs` currently exposes plain functions (no trait), called directly from `cli/download_command.rs` — nothing in `domain/` depends on it today.

Researched and rejected adopting an existing crate for the persisted-queue/event mechanics (`apalis`, `omniqueue`, `underway`, `outbox-core`, `honker`, `tokio::sync::broadcast`) — each either requires migrating off `rusqlite`'s blocking API, has no SQLite backend, ships as an alpha-stage native SQLite extension, or has no persistence at all. None fit this project's constraints without a migration cost or deployment risk disproportionate to what's needed here.

## Goals / Non-Goals

**Goals:**
- A generic, persisted, asynchronous domain-event mechanism: publish is durable and decoupled from consumption; multiple independent subscribers can react to the same event type; a slow or failing subscriber never affects the request that published the event.
- A generic, persisted, restart-safe task-scheduling mechanism: a task runs at or after a specific time, is retried a bounded number of times, and survives an unclean daemon restart.
- A concrete `sync_playlist` operation, wired through both mechanisms, that keeps a `videos` table in sync with a YouTube playlist's actual contents, first on creation and then on a recurring interval, indefinitely.
- Keep the domain-event mechanism free of any playlist/task-specific knowledge, since it is intended to be extracted as a standalone library later.

**Non-Goals:**
- Downloading videos, or anything that consumes `videos.status = PENDING` — a future change.
- Multi-process or distributed execution — this is one poller of each kind, in one process, matching the single-container deployment.
- Exactly-once delivery/execution — both mechanisms are at-least-once; correctness depends on subscribers and task handlers being idempotent.
- Actually extracting the domain-event mechanism into a separate crate now — only keeping it structured so that's feasible later.

## Decisions

### 1. Domain events and tasks are two separate mechanisms, not one shared queue
Each gets its own table and its own poll loop, even though both end up with a similar status/retries/timestamps shape.
**Why:** the domain-event mechanism is meant to be extracted as a standalone, domain-agnostic library later (explicit user intent); the task mechanism is inherently yarrtube-specific (sync interval, `sync_playlist` dispatch). Sharing physical storage would permanently couple the two, making later extraction a matter of untangling shared state rather than lifting out something self-contained.
**Alternative considered:** collapse both into one generic "durable retryable job" table (raised during design as a DRY opportunity, since the two are structurally identical). Rejected for the reason above, and because their consumption semantics genuinely differ: events fan out to N independent idempotent subscribers per row, tasks dispatch to exactly one handler per row.

### 2. Hand-rolled over an existing crate
**Why:** see Context — every candidate crate either forces migrating `rusqlite` (blocking) to `sqlx` (async), lacks a SQLite backend, is alpha-stage native-extension software, or isn't persisted at all. The mechanism itself (poll a table, claim a row, dispatch, retry-with-counter) is small enough that hand-rolling it twice (once for events, once for tasks) is less risk than adopting a crate that only solves half the problem.

### 3. `events` table: no `run_at`, single retry counter per event
```
events: id, event_type, payload (JSON), status, retries, created_at, updated_at, last_error
```
Events are always eligible for processing as soon as they're published — only tasks support scheduling into the future, so `events` has no `run_at` column.

On publish, the domain-agnostic `DomainEventsConsumer` looks up every subscriber registered for the event's type and invokes each one in-process. If **any** subscriber fails, the event's single `retries` counter is incremented and the event stays `pending` for a full retry — **all** subscribers run again, including ones that already succeeded. After 5 failed attempts the event is logged and marked `failed` permanently.
**Why:** a per-subscriber delivery table (event x subscriber, independently tracked) was considered and would be more precise, but the user explicitly chose the simpler single-counter model and accepted its known gap (one subscriber's failure causes redundant reprocessing of others) in exchange for requiring subscribers to be idempotent — a requirement this design documents as a convention for anyone writing a subscriber, not something the framework enforces.

`DomainEventsPublisher` (the write path) and `DomainEventsConsumer` (the poll/dispatch path) are the reusable pieces; the mapping from event type to subscriber(s) is supplied by the app at startup (analogous to how `http::playlists_router` wires app-specific routes onto axum) and lives in the new `subscribers/` module, which — like `http/` and `cli/` — is a thin adapter: decode payload, call one domain-service method, nothing else.

### 4. `tasks` table: `run_at`, single consumer, fixed-delay retry, crash recovery
```
tasks: id, task_type, payload (JSON), status, retries, run_at, created_at, updated_at, last_error
```
One task dispatches to exactly one domain-service call (no fan-out). On failure, `retries` increments and the task becomes eligible again after a short fixed delay (no exponential backoff); after 5 attempts it's logged and marked `failed` permanently.

On daemon startup, any task still `running` (interrupted by an unclean shutdown) is recovered by running it through the exact same "failed attempt" path — `retries` increments, and it's dropped if that exceeds 5 — rather than a separate recovery code path. This intentionally reuses existing logic instead of adding a new state.

The mapping from task type to domain-service call, and the poll loop that claims due tasks and dispatches them, live in the new `tasks/` module — same adapter role as `subscribers/`, `http/`, `cli/`.

Both new poll loops (`DomainEventsConsumer`, task executor) run every ~5 seconds via `tokio::spawn`, matching `heartbeat_loop`'s existing shape. Five seconds is frequent enough that a "sync now" request is picked up promptly, and cheap enough against a local SQLite file that tighter polling isn't worth the complexity of a smarter wake mechanism.

### 5. Recurrence is task-driven, not event-driven
`sync_playlist` unconditionally schedules its own next run as a `sync_playlist` task at `now + YARRTUBE_SYNC_INTERVAL_SECONDS`, every time it runs, regardless of how many (if any) per-video events it published.
**Why (this was a correction made mid-design):** the original idea was for a `PlaylistSynced` event to drive rescheduling, mirroring how `PlaylistCreated` drives the first sync. That breaks the very first time a sync finds zero new videos — the common steady-state case — because zero events would be published and nothing would trigger the next attempt, silently ending the recurring cycle. Self-scheduling via a task sidesteps this: it doesn't depend on the sync's business outcome at all.

### 6. The first sync is event-triggered and synchronous within that event's processing; later syncs are task-triggered
`PlaylistCreated` → subscriber `sync_playlist_on_playlist_created` calls `PlaylistService::sync_playlist(playlist_id)` directly, as that event's own processing (not as a separately scheduled task). Every subsequent run is triggered by the recurring `sync_playlist` task instead.
**Consequence, accepted as-is:** the same `sync_playlist` method is retried under two different policies depending on how it was invoked — the event's single-counter/retry-the-whole-event policy for the first run, the task's five-attempts/fixed-delay/drop policy for every run after. This is a natural result of the design, not something this change tries to unify.

### 7. `videos` table: composite natural key, delete-on-removal, status untouched on re-sync
```
videos: playlist_id, youtube_video_id, title, status, created_at, updated_at
  PRIMARY KEY (playlist_id, youtube_video_id)
```
A natural composite key, consistent with `playlists` using its YouTube ID (not a surrogate) as its primary key. `status` is only ever written as `PENDING` in this change.

On each re-sync: videos still present are left untouched beyond a title/`updated_at` refresh (status is never reset); a video previously stored but no longer present in the source playlist is deleted from the table, and the deletion is logged. No download has happened yet in this change, so there's no local file to reconcile.

### 8. `sync_playlist` no-ops if the playlist no longer exists
Before fetching anything, `sync_playlist` checks the playlist still exists via `PlaylistRepository`. If it's gone (deleted since this run was scheduled), it does nothing — no YouTube call, no events, no next task scheduled.
**Why:** this is how a deleted playlist's recurring cycle terminates on its own, without needing an explicit "cancel this playlist's pending tasks" mechanism. `delete_playlist` still publishes `PlaylistDeleted` (per the proposal), but no subscriber reacts to it in this change — actual `videos` cleanup on deletion is deferred.

### 9. Promote `youtube_api.rs` from `shared/` to a trait-backed port
`sync_playlist` needs to inject and fake this dependency (per the project's testing convention: mock only at domain-service boundaries). Its free functions move to a trait (e.g. `YoutubePlaylistItemsRepository`) in `infrastructure/repositories/`, alongside the existing `YoutubePlaylistRepository` (existence-check) port. This is the expected/anticipated promotion path the project's own conventions already describe for `shared/` code once something needs to depend on it via DI.

### 10. Close the outbox durability gap with a real transaction
`create_playlist`'s playlist-row insert and its `PlaylistCreated` event-row insert are wrapped in a single SQLite transaction (`rusqlite::Connection::transaction`), as is `delete_playlist`'s playlist-row delete and its `PlaylistDeleted` event-row insert. This makes the outbox genuinely transactional rather than "two writes with a negligible gap between them" — cheap to do given `rusqlite` supports transactions natively, so there's no reason to accept the gap.

## Risks / Trade-offs

- **Single retry counter per event conflates independent subscribers' outcomes** (one failing subscriber causes a successful one to redundantly re-run) → mitigated by requiring subscriber idempotency; accepted explicitly by the user as a simplification for the current single-subscriber-per-event reality.
- **At-least-once, not exactly-once**, for both events and tasks (retries and crash recovery can reprocess) → mitigated by the same idempotency requirement, applied to task handlers too.
- **A permanently-dropped `sync_playlist` invocation (event or task exhausts its retries) silently ends that playlist's sync cycle** until someone notices the log and intervenes → accepted as a known limitation for this change; not solved here.
- **Two structurally similar poll loops (events, tasks) duplicate some bookkeeping code** (status/retries/timestamps, crash recovery) → accepted deliberately to preserve the event mechanism's extractability; revisit only if a third similar mechanism appears (avoid a premature shared abstraction for two call sites).
- **Deleting a playlist does not clean up its `videos` rows in this change** → rows are orphaned but inert (nothing reads them once the owning playlist is gone); a `PlaylistDeleted` subscriber to clean them up is explicitly deferred, not forgotten.

## Migration Plan

- All three new tables (`events`, `tasks`, `videos`) are created with `CREATE TABLE IF NOT EXISTS` at repository construction time, following the existing `SqlitePlaylistRepository::new` pattern — no manual migration step or schema versioning is introduced.
- No existing endpoint, CLI command, or stored data changes shape; the only observable change to existing flows is that `create_playlist`/`delete_playlist` now also publish an event.
- Rollback is a plain revert: the new tables sit unused in the SQLite file, which is itself not persisted across container recreation for this project (see `daemon` spec) — nothing to clean up.
