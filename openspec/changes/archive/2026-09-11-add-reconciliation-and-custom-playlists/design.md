## Context

Today's recurring per-playlist job (`SyncPlaylistTask`, invoked via the
existing task-scheduling/domain-events infrastructure) does exactly one
thing: diff a playlist's stored videos against YouTube's playlist-items API
and converge (add `PENDING`, hard-delete what's gone). File cleanup
(`delete_video_file`) is a one-shot reaction to a single `VideoDeleted`
event, and it locates the file to remove by re-deriving a sanitized filename
stem from the video's title and probing for an optional `[video_id]`
collision suffix (`ytdlp::resolve_collision`) — it never learns the filename
`yt-dlp` actually produced. `VideoDownloaderRepository::download` returns
only `Ok(bool)`; nothing captures the real output filename. `PlaylistId` and
`VideoId` are already unconstrained non-empty-string wrappers (not
YouTube-format-validated at the type level), and the SQLite repositories
create their tables with `CREATE TABLE IF NOT EXISTS` only — there is no
migration framework in this codebase yet. The task executor dispatches tasks
one at a time, sequentially, within a single process (`TaskExecutor::poll_once`
is a plain `for` loop), so there is no intra-process concurrency between
tasks today; HTTP handlers do run concurrently with it, since routes are
async and task execution runs via `spawn_blocking`.

See `proposal.md` for motivation. This document covers how the reconcile
task, the filename-tracking mechanism, and the custom-playlist HTTP surface
are actually built.

## Goals / Non-Goals

**Goals:**
- One recurring per-playlist task that heals YouTube-membership drift
  (`YoutubeLinked` playlists only) and filesystem drift (every playlist),
  without adding a second scheduling mechanism.
- Exact, non-fuzzy tracking of what a successful download actually wrote to
  disk, replacing sanitized-title/collision-suffix guessing everywhere it's
  used.
- A second playlist kind (`Custom`) whose video membership is entirely
  HTTP-driven, proven against the existing download/cleanup pipeline with no
  changes to that pipeline.
- Heal the download/removal race (an orphaned file left behind when a video
  is removed while its download is still running) by detection on the next
  reconcile pass, not by adding locking to `download_video`.

**Non-Goals:**
- No manual removal for a `YoutubeLinked` playlist's videos — still means
  editing the YouTube playlist itself.
- No tombstones/soft-deletes anywhere. Each playlist kind has exactly one
  writer of membership (YouTube via reconciliation, or HTTP CRUD), so a hard
  delete can never be silently resurrected by the other side.
- No per-video membership provenance — `kind` lives on the playlist, not on
  individual video rows.
- No general schema-migration framework — this change adds two
  `ALTER TABLE ... ADD COLUMN` statements, run idempotently, not a
  versioned-migrations mechanism.
- No change to yt-dlp quality/container selection, task retry counts, or the
  domain-event/task-scheduling infrastructure's own mechanics.
- No Chrome extension — only the HTTP surface it will eventually call.

## Decisions

**1. One task type, branching on playlist kind, instead of two.**
`SyncPlaylistTask` is renamed to `ReconcilePlaylistTask` and scheduled for
every playlist (today it's already scheduled for every playlist that exists,
since all playlists are `YoutubeLinked`). Each run: no-ops if the playlist no
longer exists (unchanged); if `kind == YoutubeLinked`, runs today's
membership diff unchanged; unconditionally, runs the new filesystem-diff
step (below); reschedules itself at `now + reconcile_interval_seconds`
(renamed from `sync_interval_seconds`, same default and env var pattern).
*Alternative considered:* a second, independent recurring task type just for
filesystem healing. Rejected — it would duplicate the entire
schedule/reschedule/no-op-on-deleted-playlist plumbing this task already
has, for no benefit, and a `Custom` playlist would need it scheduled at
creation time exactly like `ReconcilePlaylistTask` already is.

**2. Playlist creation still triggers one immediate reconcile pass for both
kinds**, via the same `PlaylistCreated` subscriber mechanism used today (no
kind-based branching at the subscriber level). For `Custom`, that first pass
finds no videos and no files yet and is a harmless no-op; this keeps "every
new playlist gets one immediate pass" a kind-independent invariant instead
of special-casing it.

**3. Filesystem-diff step, run inside `ReconcilePlaylistTask` for every
playlist:**
```
list files currently in the playlist's output directory
for each Downloaded video whose recorded filename is not in that listing:
    reset it (status -> Pending, clear filename and quality)
    schedule a fresh DownloadVideoTask for it directly
    (not by re-publishing VideoAdded — that event means "newly
     discovered"; a status reset isn't that, even though the
     practical effect largely rhymes)
for each file in the listing that is:
    - not a yt-dlp in-progress temp file (.part, .ytdl, and similar), and
    - not the recorded filename of any currently-Downloaded video
  delete it
```
The output directory is treated as fully system-owned, per your call: any
other file present (including a stray non-download artifact like
`.DS_Store`) is deleted too. This is an intentional, accepted trade-off —
worth a line in the README so it isn't a surprise to whoever operates this.

**4. Record the exact downloaded filename via yt-dlp's own reporting,
instead of re-deriving or globbing for it.** `ytdlp::download_video` asks
yt-dlp for its actual output filename via a single explicit `--print` field
(e.g. `after_move:filename`) and returns it. `VideoDownloaderRepository::download`'s
signature changes from `anyhow::Result<bool>` to `anyhow::Result<Option<String>>`
— `Some(filename)` on success, `None` on a clean yt-dlp failure, `Err` still
reserved for a systemic problem (yt-dlp missing, unparseable `--print`
output, etc. — the same bucket "yt-dlp missing from PATH" already lives in
today). `Video` gains `filename: Option<String>`, set alongside
`mark_downloaded` and cleared by the reset in decision 3.
*Alternative considered:* after the process exits, glob the output directory
for whatever file has the stem `resolve_collision` computed before starting
the download. Rejected — ambiguous if more than one file matches, and it's
strictly less direct than just asking yt-dlp what it actually wrote.

**5. `delete_video_file` keys off the recorded filename, not a
re-derived guess.** `FilesystemVideoFileRepository::delete`'s fuzzy
stem/collision-suffix matching (`video-cleanup`'s current spec) is replaced
by an exact-filename delete, used identically by the existing
`VideoDeleted`-triggered cleanup path and the new reconciliation orphan
sweep — one matching mechanism, two callers.

**6. `Playlist` gains `kind: PlaylistKind` (`YoutubeLinked` |
`Custom`)**, a plain enum value object shaped like the existing `Quality`
(`new`/`as_str`/`Display`), stored as a `TEXT` column. `PlaylistId` stays a
single free-form non-empty-string identity for both kinds — it's already
effectively that today (`PlaylistId::new` only rejects empty strings), so no
"internal vs. external id" split is needed. Creation validation branches by
endpoint rather than by a request field: `POST /playlists` keeps requiring
the id to resolve on YouTube (unchanged `PlaylistService::create_playlist`);
`POST /custom-playlists` requires the caller-supplied id to parse as a
well-formed UUID and not already exist, and never calls the YouTube lookup.

**7. Custom-playlist video mutation is enforced at one point.** New
`VideoService` methods (`add_video_to_custom_playlist`,
`remove_video_from_playlist`) both start by loading the target playlist and
returning a clear domain error if its `kind` isn't `Custom`. This is the
single place "no manual removal on a YouTube-linked playlist" is enforced,
rather than duplicating the check across the add and remove HTTP handlers.
Both methods otherwise reuse existing machinery unchanged: add creates a
`Pending` video and publishes `VideoAdded` (same event `sync` already
publishes, already wired to `DownloadVideoOnVideoAdded`); remove hard-deletes
the row and publishes `VideoDeleted` (same event, already wired to the
cleanup path).

**8. New `YoutubeVideoRepository` port** (sibling to the existing
`YoutubePlaylistRepository`/`YoutubePlaylistItemsRepository`), hitting the
YouTube Data API's `videos` endpoint with `part=snippet&id=<video_id>` to
confirm existence and fetch the title in a single call — same shape as the
two existing YouTube repositories (blocking `reqwest` client, API key query
param).

**9. Accept either a bare video ID or a full/short YouTube URL** in the
add-video request body, parsed via a new `VideoId::from_url_or_id`
alongside the existing bare-ID `VideoId::new`, so URL-parsing lives in the
domain's value object rather than the HTTP layer.

## Risks / Trade-offs

- [Every reconcile pass lists an entire playlist's output directory] →
  cost scales with directory size × interval frequency; acceptable at the
  personal-NAS scale this project targets, and bounded by the existing
  hourly default interval.
- [A persistently broken output directory (e.g. a permissions problem)
  makes every reconcile pass see the file as "missing" and reschedule a
  fresh download, forever] → each reset gets its own fresh 5-attempt task
  budget, so this is repeated noise rather than a crash loop; mitigate by
  logging each reset at `warn`, matching how dead-lettering is already
  logged, so it's visible rather than silent.
- [Treating the output directory as fully system-owned deletes anything a
  user drops in by hand] → accepted trade-off per your explicit choice;
  document it in the README so it isn't a surprise.
- [Renaming the task type breaks any `sync_playlist` task already persisted
  before this change deploys] → already flagged **BREAKING** in the
  proposal; the existing no-handler-registered → retry → dead-letter path
  absorbs it without crashing, at the cost of that playlist's next
  reconcile being delayed by a few short retry cycles. Restarting with an
  empty task queue avoids this entirely if the deploy window allows it.
- [Filename capture depends on yt-dlp's `--print` output format] →
  mitigated by requesting one explicit, narrow `--print` field rather than
  scraping general stdout, and treating a parse failure as the same kind of
  systemic `Err` "yt-dlp missing from PATH" already is today, rather than
  silently leaving `filename` unset.

## Migration Plan

- Add `ALTER TABLE playlists ADD COLUMN kind TEXT NOT NULL DEFAULT
  'YOUTUBE_LINKED'` and `ALTER TABLE videos ADD COLUMN filename TEXT`,
  executed right after the existing `CREATE TABLE IF NOT EXISTS` statements
  in each repository's `new()`, with the "duplicate column name" SQLite
  error swallowed so re-running against an already-migrated database stays
  idempotent — the same idempotency `CREATE TABLE IF NOT EXISTS` already
  gives the rest of the schema, no separate migration framework introduced.
- Ship as a single daemon restart, consistent with the existing `daemon`
  capability's startup-recovery story; no dual-running needed given this
  project's single-instance deployment model.
- Rollback is additive-safe (old code ignores the new columns) except that
  anything created as `Custom` becomes un-syncable garbage to a rolled-back
  binary (no `youtube_playlist_id` to look up) — acceptable since rollback
  is an emergency path, not a supported downgrade.

## Open Questions

- Exact set of URL shapes `VideoId::from_url_or_id` accepts (shorts,
  embed links, playlist-qualified watch URLs, etc.) — doesn't change the
  approach, the specs, or the task breakdown; can be settled during
  implementation.
