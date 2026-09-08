## 1. Domain events: data model and persistence

- [x] 1.1 Define a `DomainEvent` enum (`PlaylistCreated { playlist_id }`, `PlaylistDeleted { playlist_id }`, `VideoAdded { playlist_id, youtube_video_id }`) in `domain/event.rs`; verify with a unit test that each variant maps to a stable `event_type` string and a JSON payload.
- [x] 1.2 Define `EventPublisher` (publish one `DomainEvent`) and `EventRepository` (CQS: insert pending event, list events eligible for another attempt, mark an event done, mark an event failed-or-retry) ports in `infrastructure/repositories/`, and implement `SqliteEventRepository` against a new `events` table created with `CREATE TABLE IF NOT EXISTS`, mirroring `SqlitePlaylistRepository::new`; verify with in-memory-SQLite tests covering insert, list-pending, and mark-done.
- [x] 1.3 Implement retry bookkeeping in `SqliteEventRepository`: a failed attempt increments `retries` and keeps status `pending` until 5 attempts, then marks `failed` with the recorded error; verify with a test asserting the status/retries transitions across repeated failures, including the fifth attempt dropping the event.

## 2. Domain events: subscriber registry and dispatch

- [x] 2.1 Define an `EventSubscriber` trait (handles one event's raw JSON payload, returns `anyhow::Result<()>`) in `infrastructure/repositories/` alongside the event ports; verify it compiles against a hand-written fake subscriber in a test.
- [x] 2.2 Implement a `DomainEventsConsumer` that, given a subscriber registry (`event_type -> Vec<subscriber>`) and an `EventRepository`, polls for pending events and invokes every registered subscriber for each; verify with fake subscribers covering: one subscriber succeeding, multiple subscribers all invoked, one subscriber failing causing the whole event (all subscribers) to retry, and an event with no registered subscribers succeeding as a no-op.
- [x] 2.3 Wrap `DomainEventsConsumer` in a `tokio::spawn`'d poll loop on a ~5 second interval, matching `heartbeat_loop`'s existing shape (composition only; wiring is verified in section 9).

## 3. Task scheduling: data model and persistence

- [x] 3.1 Define a task representation (initially only `SyncPlaylist { playlist_id }`) in `domain/task/`, one file per concept (`domain/task/task.rs`, `domain/task/errors.rs`); verify with a unit test constructing a task and serializing its payload.
- [x] 3.2 Define a `TaskRepository` port (CQS: schedule a task with a `run_at`, list tasks eligible to run now, mark a task running, mark done, mark failed-or-retry) in `infrastructure/repositories/`, and implement `SqliteTaskRepository` against a new `tasks` table; verify with in-memory-SQLite tests mirroring the events tests: schedule/claim/done, and failed-then-dropped-after-5-attempts with the fixed retry delay.
- [x] 3.3 Implement startup recovery: a `TaskRepository` method that finds every task still `running` and applies the same failed-attempt handling as 3.2; verify with a test that seeds a `running` task and asserts it is recovered per the retry policy (retried if under the limit, dropped if not).

## 4. Task scheduling: executor and dispatch

- [x] 4.1 Define a `TaskHandler` trait (handles one task's raw JSON payload) in `infrastructure/repositories/` alongside the task ports; verify it compiles against a fake handler in a test.
- [x] 4.2 Implement a task executor that, given a handler registry (`task_type -> handler`) and a `TaskRepository`, polls for eligible tasks and dispatches each to its registered handler; verify with a fake handler covering: success, a failure that becomes eligible again after the fixed delay, and a failure that exhausts 5 attempts and is dropped.
- [x] 4.3 Wrap the executor in a `tokio::spawn`'d poll loop on a ~5 second interval (composition only; wiring is verified in section 9).

## 5. Videos

- [x] 5.1 Define a `Video` entity and its value objects in `domain/video/` (one file per concept, matching the playlist aggregate's shape), keyed by `(playlist_id, youtube_video_id)`, with a `VideoStatus` value that is `Pending` for now; verify with a unit test building a `Video`.
- [x] 5.2 Define a `VideoRepository` port (CQS: upsert a video, list videos for a playlist, delete videos for a playlist whose YouTube ID is not in a given current set) in `infrastructure/repositories/`, and implement `SqliteVideoRepository` against a new `videos` table; verify with in-memory-SQLite tests covering upsert leaving an existing video's status untouched, and the "delete videos not in the given set" query.

## 6. YouTube playlist items port

- [x] 6.1 Promote `infrastructure/shared/youtube_api.rs`'s pagination/fetch logic into a trait-backed port `YoutubePlaylistItemsRepository` (read: list a playlist's current videos) in `infrastructure/repositories/`, implemented against the existing YouTube Data API v3 call; verify the existing `mockito`-based pagination tests still pass against the new port implementation.
- [x] 6.2 Confirm `cli/download_command.rs` still behaves the same, either by switching it to the new port or leaving it on the underlying function directly; verify existing CLI download tests still pass.

## 7. Playlist service: sync_playlist and event publishing

- [x] 7.1 Inject `EventPublisher` into `PlaylistService`; have `create_playlist` publish `PlaylistCreated` only on the `Created` outcome (not `AlreadyExisted`), in the same SQLite transaction as the playlist insert; verify with a service test asserting the event publishes only on genuine creation, and a repository-level test asserting both writes commit together.
- [x] 7.2 Have `delete_playlist` publish `PlaylistDeleted` on successful deletion, in the same transaction as the delete; verify with a service test.
- [x] 7.3 Add `sync_playlist(playlist_id)` to `PlaylistService`, injected with `VideoRepository`, `YoutubePlaylistItemsRepository`, `TaskRepository`, and `EventPublisher`. It SHALL: no-op if the playlist no longer exists; otherwise fetch current videos, upsert new ones as `PENDING`, delete stored videos no longer present (logging each), publish one `VideoAdded` event per newly added video, and always schedule the next `SyncPlaylist` task at `now + interval` (reading `YARRTUBE_SYNC_INTERVAL_SECONDS`, defaulting to 3600). Verify with service tests covering: no-op on a missing playlist, new videos persisted and events published, removed videos deleted and logged, and the next sync always being scheduled even when nothing changed.

## 8. Subscribers and tasks wiring

- [x] 8.1 Add `subscribers/sync_playlist_on_playlist_created.rs`: an `EventSubscriber` that decodes `PlaylistCreated`'s payload and calls `PlaylistService::sync_playlist`; verify with a test (using a fake or test-double service) asserting the correct playlist ID is synced.
- [x] 8.2 Add `subscribers/mod.rs` as the registry wiring `PlaylistCreated -> [sync_playlist_on_playlist_created]`, ready to be handed to `DomainEventsConsumer` at composition time.
- [x] 8.3 Add `tasks/sync_playlist_task.rs`: a `TaskHandler` that decodes a `SyncPlaylist` task's payload and calls `PlaylistService::sync_playlist`; verify with a test asserting the correct playlist ID is synced.
- [x] 8.4 Add `tasks/mod.rs` as the registry wiring `SyncPlaylist -> sync_playlist_task`, ready to be handed to the task executor at composition time.

## 9. Daemon wiring

- [x] 9.1 In `serve.rs`, construct `SqliteEventRepository`, `SqliteTaskRepository`, and `SqliteVideoRepository` alongside the existing playlist repository and the promoted YouTube playlist-items port, and wire `PlaylistService` with its new dependencies.
- [x] 9.2 Run startup task recovery (3.3) before starting the task executor poll loop, following the same startup-sequencing pattern as `run_startup_database_check`; verify by seeding a `running` task in the database file and confirming a fresh `serve` startup recovers it before the executor begins polling.
- [x] 9.3 Spawn the `DomainEventsConsumer` (2.3) and task executor (4.3) poll loops alongside the existing `heartbeat_loop`, registering `subscribers::registry()` and `tasks::registry()`; verify by running the daemon locally, creating a playlist via `POST /playlists`, and confirming videos appear in the `videos` table shortly after without delaying the HTTP response.
- [x] 9.4 Document `YARRTUBE_SYNC_INTERVAL_SECONDS` in `.env.example` and the README's configuration section; verify by reading the updated files.

## 10. End-to-end verification

- [ ] 10.1 Manually verify the full loop against a real (or small test) YouTube playlist: create a playlist and confirm videos appear with status PENDING; remove a video from the source playlist and confirm the next sync deletes it and logs the deletion; kill the daemon mid-sync and confirm the next startup recovers cleanly rather than getting stuck.
- [x] 10.2 Run `cargo test` and `cargo clippy` and confirm both pass cleanly.
