## Why

`VideoRepository` and `PlaylistRepository` currently embed decisions that belong in
the domain layer: `VideoRepository::delete_not_in` decides *which* rows survive
via a `NOT IN` SQL predicate instead of being told which entity to delete, its
`upsert` bakes in a conflict-merge policy ("keep status, overwrite title"), and
`PlaylistRepository::insert_with_event`/`delete_with_event` decide to publish a
domain event as a side effect of a write. This makes repositories bigger and
harder to fake than plain collections, and it's about to get worse: the
upcoming video-deletion work needs the domain to know exactly which video is
being removed so it can react per-video (see the sibling change adding
`VideoDeleted`) — `delete_not_in`'s bulk, repo-decided semantics can't support
that. Flattening repositories to `find`/`save`/`delete`/`list` now, with the
domain making every decision, unblocks that work cleanly and matches how
`VideoService` already publishes `VideoAdded` today.

## What Changes

- `VideoRepository`: replace `delete_not_in(playlist_id, current_ids)` with
  `delete(playlist_id, video_id)` — `VideoService::sync_playlist_videos`
  decides which videos are removed (it already iterates them) and deletes
  each one explicitly.
- `VideoRepository`: replace `upsert(video)` with `save(video)` — a plain
  insert/replace with no conflict-merge policy. `VideoService` decides
  per-video whether to create a new record or leave an existing one alone
  (per the already-documented "existing record's status is left unchanged"
  rule), constructing the exact `Video` value to persist itself.
- `PlaylistRepository`: replace `insert_with_event`/`delete_with_event` with
  plain `insert(playlist)`/`delete(playlist_id)`. `PlaylistService` gets an
  injected `Arc<dyn EventPublisher>` (mirroring `VideoService`) and calls
  `.publish(&event)` itself, as a separate step after the repository write.
  **BREAKING (internal only)**: this drops the single-SQLite-transaction
  atomicity between a playlist write and its event row, previously
  documented in `CLAUDE.md`. The accepted risk (a crash in the narrow window
  between the two writes drops the event) already exists today for
  `VideoAdded`, which is published as a bare non-transactional call; this
  makes playlists consistent with that existing precedent rather than
  introducing a new risk class.
- Update `CLAUDE.md`'s architecture notes to remove the now-inaccurate
  "playlists and events share one connection so writes can be
  transactional — don't split it apart" guidance.

No observable behavior changes: every existing `playlist-sync`,
`playlist-crud`, and `domain-events` requirement (new videos stored PENDING,
existing videos' status left untouched, removed videos deleted, events
durably persisted before the publishing call returns, etc.) continues to
hold — only *where* the decision is made and *how* it's persisted changes.
This is a pure internal refactor, so no spec deltas accompany it
(`skip_specs: true`).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — behavior is unchanged; only internal repository/service structure
changes. See `design.md` for the mechanics.

## Impact

- `src/infrastructure/repositories/sqlite_video_repository.rs` (and its
  `FakeVideoRepository`): port and implementation change.
- `src/infrastructure/repositories/sqlite_playlist_repository.rs` (and its
  `FakePlaylistRepository`): port and implementation change.
- `src/domain/video/service.rs`: `sync_playlist_videos` takes over deciding
  which videos are new/unchanged/removed and issues per-video repository
  calls.
- `src/domain/playlist/service.rs`: gains an `Arc<dyn EventPublisher>`
  dependency; `create_playlist`/`delete_playlist` publish events themselves.
- `serve.rs::build_application()`: wiring for `PlaylistService::new` gains
  the event publisher argument.
- All unit tests exercising `delete_not_in`, `upsert`, `insert_with_event`,
  `delete_with_event` on either repository or their fakes.
- `CLAUDE.md`: remove the connection-sharing/atomicity note.
