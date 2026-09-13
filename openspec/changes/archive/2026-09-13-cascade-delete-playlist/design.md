## Context

See `proposal.md` - Why. Two existing pieces of plumbing are directly relevant:

- Domain events are an outbox: `EventPublisher::publish` just inserts a
  `pending` row into a SQLite `events` table; `DomainEventsConsumer::run`
  polls it every 5s (`BACKGROUND_POLL_INTERVAL` in `serve.rs`) and dispatches
  to subscribers. Scheduled tasks (`TaskRepository`/`TaskExecutor`) work the
  same way, polled on the same 5s interval. A subscriber reacting to an
  event by scheduling a task is therefore at least one, usually two, 5s poll
  cycles removed from the triggering write.
- `ReconcilePlaylistTask` / `VideoService::reconcile_playlist` and
  `DeleteVideoFileOnVideoDeleted` / `VideoService::delete_video_file` both
  resolve the playlist via `playlist_repository.find(id)` at execution time
  and silently no-op if it's gone. That's the right behavior for those two
  (a still-existing playlist reconciling itself; a video removed from a
  still-existing playlist), but it means neither can be reused as-is for
  "the playlist just got deleted" - by construction the playlist row won't
  be there anymore.

## Goals / Non-Goals

**Goals:**
- No video record is ever observable referencing a playlist that doesn't
  exist, not even for one poll cycle.
- A playlist's on-disk directory is fully removed after deletion, without
  needing to track individual filenames.

**Non-Goals:**
- General event/task pipeline changes (e.g. removing the poll-based outbox,
  making `EventPublisher::publish` synchronous). Out of scope; the design
  below works within the existing pipeline by being deliberate about which
  part of the work needs to skip it.
- Retroactively cleaning up playlists deleted before this change ships
  (pre-existing orphaned video rows/files, if any exist in a running
  deployment). Not addressed here.

## Decisions

### Video-record cleanup runs synchronously inside `delete_playlist`, not through the event/task pipeline

A video row referencing a nonexistent playlist is an invalid state, unlike
a playlist with zero videos (which is simply early in its life - the same
shape `PlaylistCreated` -> `ReconcileOnPlaylistCreated` already produces
for a moment). Given the outbox's ~5-10s real lag, and that playlist IDs
are not tombstoned (deleting and re-adding the same YouTube playlist, or
reusing a custom playlist's UUID, are both normal, unprevented actions),
routing this through the event pipeline would create a real window where
stale video rows could reattach to a newly (re)created playlist with the
same ID.

`PlaylistService::delete_playlist` therefore takes on a `VideoRepository`
dependency (mirroring the existing reverse direction - `VideoService`
already depends on `PlaylistRepository`) and, within the same call: finds
the playlist (as it already does), bulk-deletes every video row for it via
a new `VideoRepository::delete_all_for_playlist` method, deletes the
playlist row, then publishes `PlaylistDeleted`.

**Alternative considered:** an orchestration layer above both services
(fetch videos, delete them, then delete the playlist) that would avoid
`PlaylistService` depending on `VideoRepository`. Rejected because this
codebase's `http`/`cli` adapters are documented to "call one domain method
... nothing more" (`CLAUDE.md`), and no orchestration layer above
per-aggregate services exists yet; introducing one for a single call site
is more new machinery than the dependency it's avoiding.

### File/directory cleanup stays on the existing event -> subscriber -> task pipeline

Unlike video rows, nothing in the domain depends on a stale file being gone
within a specific window - reconcile already tolerates and heals filesystem
drift on its own schedule. So this part keeps the same shape as
`VideoDeleted` -> `DeleteVideoFileOnVideoDeleted` -> `DeleteVideoFile`:
`PlaylistDeleted` gains a `path` field (the playlist row won't exist by the
time anything reacts to it, so the path has to travel in the payload - the
same reason `path` can't be re-looked-up later), a new subscriber
(`playlist_deleted` -> schedule a task) and a new task
(`DeletePlaylistFiles { playlist_id, path }`) whose handler removes
`videos_path/path` recursively and no-ops if it's already gone.

Recursive removal (rather than deleting files by recorded filename one at a
time, the way `DeleteVideoFile` does) is safe here specifically *because*
video rows are already gone by the time this event is published - there's
nothing left to distinguish "ours" from "not ours" by filename anyway, and
path uniqueness (next decision) guarantees the directory holds nothing but
this playlist's own files.

### Playlist paths must be unique across playlists, enforced at creation

Both `create_playlist` and `create_custom_playlist` gain a check: reject
(400) if the requested path is already the stored path of a different
existing playlist. This is checked after the existing same-ID idempotency
lookup (so re-submitting a playlist's own existing path is never rejected)
and alongside the other structural validations (name/path shape/quality),
before any YouTube lookup - consistent with how those already behave.

This closes a latent bug independent of deletion: two playlists sharing a
path today would already cause each one's `reconcile_filesystem` to treat
the other's downloaded files as orphans and delete them, causing endless
redownload/delete thrash. Enforcing uniqueness fixes that and is exactly
the precondition that makes recursive directory removal on delete safe.

Implementation-wise, the uniqueness check needs to look across all stored
playlists' paths; the existing `PlaylistRepository::list()` is sufficient
for this (no new query method required) given the expected scale of a
personal-use playlist tracker.

## Risks / Trade-offs

- [`PlaylistService` now depends on `VideoRepository`, a new
  cross-aggregate coupling] -> Accepted deliberately (see Decisions above);
  it mirrors the dependency `VideoService` already has on
  `PlaylistRepository` in the other direction, and closing an invariant gap
  is exactly the kind of cross-aggregate concern this codebase already
  handles via direct dependency for reads.
- [`PlaylistDeleted`'s payload shape changes (`path` added); any code
  deserializing old pending event rows across a deploy would break] ->
  No production deployments exist yet for this hobby project and the
  events table is a transient queue (rows live seconds, not persisted
  history), so this is not a real migration concern here.
- [Recursive directory removal deletes anything in that directory, not just
  recognized video files] -> Mitigated by the new path-uniqueness rule: as
  long as it holds, nothing but this playlist's own output ever lives at
  that path.
