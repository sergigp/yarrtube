## Context

See `proposal.md` - Why. This builds directly on `refactor-task-event-repositories`
(assumed already merged): `TaskRepository` is CRUD, `ScheduledTask` is a
domain entity carrying `retries`, and `TaskExecutor` drives
`find → start → dispatch → update/delete/dead_letter`. This change follows
that same repository/entity pattern for `Video`, and adds one more piece
that pattern didn't need yet: giving a task handler visibility into how many
attempts a task has left.

`Video` (`src/domain/video/video.rs`) currently has one status, `Pending`,
with a comment noting more arrive once something consumes it — this change
is that consumer. `VideoRepository::upsert` intentionally preserves status
on conflict (sync's merge semantics) and stays as-is; it does not become the
way status changes.

The existing `YoutubeDownloaderClient` (`infrastructure/client/`) is
CLI-only — constructed directly by `cli::download_command`, never injected
into `domain/`. It downloads a whole `Vec<Video>` (its own DTO, from
`infrastructure/shared/youtube_api::Video`, carrying a `url` field) into one
directory, printing progress as it goes. The event-driven path downloads
exactly one domain `Video` (which has no `url` field, only a `video_id`) at
a time, and is injected into `VideoService` — per the project's
client-vs-repositories split, that means a new port in
`infrastructure/repositories/`, not reuse of `YoutubeDownloaderClient` itself.

## Goals / Non-Goals

**Goals:**
- Extend `Video`/`VideoStatus` and `VideoRepository` following the CRUD +
  entity-transition pattern, with no `mark_*`-shaped repository methods.
- Reuse the actual `yt-dlp` process invocation between the CLI path and the
  new event-driven path instead of duplicating `Command::new("yt-dlp")`.
- Give `DownloadVideoTask` just enough information to distinguish "this
  failure still has retries left" from "this was the last attempt",
  without teaching generic task infrastructure anything about videos.

**Non-Goals:**
- No change to the CLI `download` command's behavior or output format.
- No error-message persistence on `Video` (status only, per the decision in
  chat — a future capability can add that if a status/list endpoint ever
  needs it).
- No concurrency changes: `TaskExecutor` still processes one eligible task
  at a time, so videos download sequentially, same as the CLI command does
  today.
- No change to how playlist names are validated (already filesystem-safe,
  enforced by `playlist-crud`) — this change only consumes that guarantee.

## Decisions

**Status transitions live on `Video`, not `VideoRepository`.**
```rust
impl Video {
    pub fn start_download(self, now: DateTime<Utc>) -> Self { /* -> InProgress */ }
    pub fn mark_downloaded(self, now: DateTime<Utc>) -> Self { /* -> Downloaded */ }
    pub fn mark_errored_retrying(self, now: DateTime<Utc>) -> Self { /* -> ErroredRetrying */ }
    pub fn mark_errored(self, now: DateTime<Utc>) -> Self { /* -> Errored */ }
}
```
`VideoRepository` gains `find(playlist_id, video_id) -> Option<Video>` and
`update(&Video) -> Result<()>`. `VideoService.download_video` reads, calls
the right transition, writes — it never asks the repository to "mark"
anything, matching the pattern `refactor-task-event-repositories` establishes
for tasks/events.

**`TaskHandler::handle` gains attempt information; `DownloadVideoTask`
decides the status, not the executor.** `ScheduledTask` (from the other
change) already carries `retries`. `TaskExecutor` passes something like
`is_last_attempt: bool` — computed from `retries` and the existing
`MAX_ATTEMPTS` the entity already owns — into `handle()`. `DownloadVideoTask`
uses it purely to pick `mark_errored_retrying` vs `mark_errored`; the
executor itself still doesn't know anything about videos. `SyncPlaylistTask`
receives the same parameter and ignores it. Alternative considered: give
`TaskHandler` a second `on_permanently_failed` hook instead — rejected
because it requires `TaskRepository::dead_letter`'s caller to branch and
invoke a second method, splitting "what happens on final failure" across two
call sites instead of one `if` inside the handler that already owns that
decision.

**A new `infrastructure/repositories/` port for single-video download**,
separate from the CLI's `YoutubeDownloaderClient`. Something like:
```rust
pub trait VideoDownloaderRepository: Send + Sync {
    fn download(&self, video_url: &str, output_dir: &Path) -> anyhow::Result<bool>;
}
```
returning `Ok(true)`/`Ok(false)` for yt-dlp's exit status (matching the
existing convention in `youtube_downloader_client.rs`), `Err` only for a
systemic problem (yt-dlp missing from `PATH`). The actual
`Command::new("yt-dlp").arg(url).current_dir(dir).status()` call moves into
`infrastructure/shared/` (e.g. `ytdlp.rs`) as a plain function with no trait
around it — both this new repository and `YoutubeDownloaderClient::download_all`
call it, so the process-invocation logic exists exactly once.

**Video URL is constructed from `video_id`, not re-fetched from YouTube.**
`https://www.youtube.com/watch?v={video_id}` — the ID was already resolved
once during sync; there's no reason to spend another YouTube Data API call
just to re-derive a URL that's a pure string format.

**Output directory: `<YARRTUBE_VIDEOS_PATH>/<playlist.name>/`.**
`VideoService.download_video` looks up the playlist via the
`PlaylistRepository` it already holds to get `playlist.name`, and joins it
with a new `videos_path: String` constructor argument (mirroring how
`sync_interval_seconds` is already threaded through as a plain config
value). Directory creation reuses the existing `ensure_output_dir`-style
`create_dir_all` behavior from the CLI client (also moved to
`infrastructure/shared/`).

**Skip-if-gone guards mirror `playlist-sync`'s existing pattern.**
`sync_playlist_videos` already no-ops when `playlist_repository.find`
returns `None` (see `playlist-sync` - "Sync Skipped For a Deleted
Playlist"). `download_video` applies the same idea twice: no playlist found,
or no video found (e.g. removed by a sync that raced the download task) —
both return `Ok(())` without calling `yt-dlp` or touching status, so the
scheduled task is marked done rather than retried for a video that no
longer needs downloading.

## Risks / Trade-offs

- [Risk] Between `VideoRepository::find` and `::update`, another process
  could mutate the same row (e.g. a concurrent sync's `upsert` renaming the
  playlist, or deleting the video). → Mitigation: same as
  `refactor-task-event-repositories`'s equivalent risk — one sequential
  `TaskExecutor` loop, one `Mutex<Connection>` behind SQLite; no new
  concurrent-access pattern introduced. `upsert`'s conflict clause already
  only touches `title`/`updated_at`, never `status`, so a racing sync can't
  clobber a download's status transition either way.
- [Risk] `TaskHandler::handle`'s signature changes a second time (once for
  the CRUD refactor's `ScheduledTask`, again here for attempt info) if the
  two changes land far apart. → Mitigation: land
  `refactor-task-event-repositories` first (already sequenced this way);
  this change's task list includes updating `SyncPlaylistTask`'s call sites
  once, not twice.
- [Risk] A playlist name that was valid at creation time (filesystem-safe)
  is trusted as a directory name here without re-validating. →
  Mitigation: accepted — `playlist-crud` already rejects unsafe names at
  creation, and playlist names are immutable after creation (no rename
  operation exists), so there's no path for an unsafe name to reach this
  code.

## Migration Plan

No database migration for `videos` (status is still a `TEXT` column, just
with new valid values) or dead-letter tables. Existing videos already stored
as `PENDING` are picked up the same way new ones will be once the next sync
runs and republishes... actually they won't: `VideoAdded` only fires for
videos not previously stored (see `playlist-sync` - "New Video
Persistence"), so **videos already stored as `PENDING` before this change
ships will not be retroactively downloaded** — only videos added after
deployment trigger a download. This is called out explicitly as an accepted
gap, not silently glossed over; a manual `yarrtube download` run covers any
backlog if needed.

## Open Questions

None — every decision above was either resolved in discussion or follows
directly from `refactor-task-event-repositories`'s established pattern.
