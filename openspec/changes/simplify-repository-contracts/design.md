## Context

See `proposal.md` - Why for the motivation. Two repositories currently embed
decisions:

- `VideoRepository::delete_not_in(playlist_id, current_ids)` — decides which
  rows survive via a `NOT IN` predicate.
- `VideoRepository::upsert(video)` — decides a conflict-merge policy in SQL
  (`ON CONFLICT DO UPDATE SET title=..., updated_at=...`, deliberately not
  touching `status`).
- `PlaylistRepository::insert_with_event`/`delete_with_event` — decide to
  publish a domain event as part of a write, and wrap both writes in one
  SQLite transaction to make that atomic. `build_application()` currently
  gives `SqlitePlaylistRepository` and `SqliteEventRepository` the *same*
  `Arc<Mutex<Connection>>` specifically so that transaction can span both
  tables (`serve.rs:109-119`).

`EventRepository` and `TaskRepository` are already collection-shaped
(`find`/`list_eligible`/`update`/`delete`/`dead_letter`, all just persisting
values the domain already decided) and are out of scope here.

`VideoService` already shows the target shape for event publishing: it holds
an injected `Arc<dyn EventPublisher>` and calls `.publish(&event)` directly
after `video_repository.upsert(...)`, with no transaction. `PlaylistService`
needs to move to that same shape.

## Goals / Non-Goals

**Goals:**
- `VideoRepository` and `PlaylistRepository` expose only plain
  collection-style operations: `find`, `save`, `delete`, `list`.
- Every decision about *what* to persist and *whether* to publish an event
  is made in `VideoService`/`PlaylistService`, never inside a repository.
- No repository other than the event repository ever touches the events
  table or knows `DomainEvent` exists.
- No observable behavior change to any existing spec requirement.

**Non-Goals:**
- Building the `VideoDeleted` event or its subscriber/task (separate,
  sibling change — this change only unblocks it).
- Changing `EventRepository`/`TaskRepository`, which are already
  collection-shaped.
- Reintroducing atomicity between a playlist write and its event via some
  other mechanism (e.g. a shared unit-of-work/transaction abstraction). The
  user has explicitly accepted the small crash-window risk instead, matching
  what already exists for `VideoAdded`.
- Changing `VideoStatus` or adding new statuses.

## Decisions

**`delete_not_in` → `delete(playlist_id, video_id)`.** `VideoService::sync_playlist_videos`
already loops over `stored_videos` to log which ones are gone; it now calls
`video_repository.delete(&id, &stored.video_id)` for each one it identifies,
instead of handing the whole "current" set to the repository and letting a
`NOT IN` clause decide. Rejected alternative: a `delete_many(playlist_id,
&[VideoId])` batch method — not needed, since sync already deletes at most a
handful of videos per run and per-video `delete` reads more directly as "the
domain named exactly what to remove."

**`upsert` → `save`, decision moves into `sync_playlist_videos`.** `save`
does a plain insert-or-full-replace with no special-cased columns — whatever
`Video` the domain hands it is what gets persisted. `sync_playlist_videos`
now does, per video from YouTube:
- not found via `video_repository.find` → build a new `Video::create(...)`
  (status `PENDING`) and `save` it, then publish `VideoAdded` — same as
  today.
- found → build an updated value (existing `status`/`created_at` kept,
  `title`/`updated_at` refreshed from the synced data) and `save` it. This
  preserves today's actual behavior (title is refreshed on every sync,
  status is preserved) while making that merge an explicit, visible decision
  in the domain instead of an SQL `ON CONFLICT` clause.

**`update` is kept, separate from `save`.** `download_video` calls
`video_repository.update(...)` to persist a status transition after
confirming with `find` that the video still exists at the *start* of the
attempt, but never re-checks before its final write. `update` only touches a
row that still exists (`UPDATE ... WHERE ...`, 0 rows affected if the video
was deleted in the meantime); `save` would be an upsert and could
resurrect a deleted video's row if the timing lined up. Collapsing `update`
into `save` was considered and rejected — it would quietly change this
existing (accidental but safe) protection into a correctness bug, which is
out of scope for a refactor that promises no behavior change.

**`PlaylistRepository` event publishing moves to `PlaylistService`.**
`PlaylistService` gains `Arc<dyn EventPublisher>` (same port `VideoService`
already uses). `create_playlist`/`delete_playlist` call
`self.repository.insert(&playlist)` / `.delete(&id)` and then
`self.event_publisher.publish(&event)` as two plain sequential calls, no
transaction. `PlaylistRepository::new` no longer needs to know about the
events table at all (drops its `create_events_table` call and the
`sqlite_event_repository` import).

**`SqlitePlaylistRepository` gets its own connection.** Since it no longer
shares a transaction with `SqliteEventRepository`, `build_application()`
stops handing them the same `Arc<Mutex<Connection>>`; `SqlitePlaylistRepository::new`
is simplified to take a plain `Connection` (owning its own `Mutex<Connection>`
internally), matching `SqliteVideoRepository::new`'s existing shape, instead
of an `Arc<Mutex<Connection>>` sized for sharing with nothing.

## Risks / Trade-offs

- **[Risk] Losing playlist/event write atomicity** → a process crash between
  the playlist write and the event publish call drops the event, leaving a
  playlist that exists but never synced. **Mitigation**: accepted by the
  user as a small, already-precedented risk (same failure mode already
  exists for `VideoAdded`); no mitigation added in this change.
- **[Risk] Behavior drift while re-deriving `upsert`'s merge in the domain**
  → easy to accidentally drop the "preserve status" or "refresh title" half
  of the existing merge while rewriting it by hand. **Mitigation**: existing
  repository/service unit tests already assert both halves (see
  `sqlite_video_repository.rs` tests `it_should_leave_the_status_untouched_when_upserting_an_existing_video`
  and the title-refresh case) — keep them passing against the new `save`
  call sites, updating only their setup (`upsert` → `find`+`save` sequence)
  where it references the removed method.

## Migration Plan

Purely internal; no data migration, no deploy-order concerns. Land as one PR
per the tasks below, running `cargo fmt`/`clippy`/`test` throughout since
there are no spec-level acceptance criteria to check against. No rollback
plan beyond a normal revert — no schema or data shape changes.
