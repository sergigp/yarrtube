## Context

See `proposal.md` - Why. Relevant existing mechanics this builds on:

- `DomainEvent` (`src/domain/event/domain_event.rs`) is a closed enum with an
  `event_type()`/`payload()` pair per variant; `Task` (`src/domain/task/task.rs`)
  is the same shape. Both are registered by string key in
  `subscribers::registry`/`tasks::registry`.
- By the time an event is processed (asynchronously, after the publishing
  transaction/call already returned), the video row it describes is already
  gone — `sync_playlist_videos` calls `video_repository.delete` right before
  publishing. Anything a subscriber or task needs about the deleted video
  must travel in the event/task payload; nothing can be looked up afterward.
- The actual on-disk filename isn't persisted anywhere. `yt-dlp` picks the
  extension, and `resolve_collision` (`src/infrastructure/shared/ytdlp.rs`)
  may have saved it as `{sanitized title} [{video_id}].{ext}` instead of
  `{sanitized title}.{ext}` if another file with the same stem already
  existed in that output directory at download time. Locating a video's file
  later means searching its playlist's output directory for either form.
- `VideoService` already holds `videos_path` and resolves a playlist's
  output directory as `videos_path/playlist.name` (see `download_video`).

## Goals / Non-Goals

**Goals:**
- A removed video's downloaded file gets cleaned up automatically, through
  the same event → subscriber → task pipeline `VideoAdded` already uses.
- Deletion is best-effort and idempotent: missing playlist, missing file, or
  a retry after partial failure are all safe no-ops, never errors that
  dead-letter the task unnecessarily.

**Non-Goals:**
- Closing the in-flight-download-vs-removal race (see proposal's "Accepted,
  deliberately unaddressed race"). No soft-delete/tombstone status, no
  re-check inside `download_video`'s completion path.
- Rebuilding `sync_playlist_videos` into a full reconcile loop — noted in
  the proposal as future work, out of scope here.
- Disambiguating two distinct videos in the same playlist that happen to
  sanitize to an *identical* title and never collided at download time (see
  Risks below) — there's no persisted data that could disambiguate them.

## Decisions

**`VideoDeleted` carries `title` and `was_downloaded`, not just IDs.** Unlike
`VideoAdded`, whose subscriber can still `find` the video (it's not gone),
`VideoDeleted` fires after the row is already deleted. The subscriber needs
`title` to eventually locate the file, and `was_downloaded` to decide
whether to schedule anything at all. `was_downloaded` is a plain bool
(`stored.status == VideoStatus::Downloaded` at the moment of deletion)
rather than the raw status string — the only distinction that matters
downstream is "might a file exist," not which non-downloaded state the
video was in.

**New `VideoFileRepository` port, separate from `VideoDownloaderRepository`.**
Downloading shells out to `yt-dlp`; deleting is pure filesystem search and
removal — different implementations and no reason to share a trait. Shape:
`delete(output_dir: &Path, filename_stem: &str, video_id: &str) -> anyhow::Result<bool>`,
returning whether a file was found and deleted. Matches an entry whose file
stem is exactly `filename_stem` or `{filename_stem} [{video_id}]`, deleting
every match (in the ordinary case there's at most one). A missing
`output_dir` is treated as "nothing to delete," not an error. Lives in
`infrastructure/repositories/video_file_repository.rs` per the rust-architect
port-placement rule (a domain service injects it), with a `FakeVideoFileRepository`
alongside it for tests, mirroring `youtube_video_downloader_repository.rs`.

**`VideoService` gains the new port and a `delete_video_file` method,**
rather than a separate service. It already owns `videos_path` and the
playlist-lookup-then-resolve-output-dir logic `download_video` uses;
`delete_video_file(playlist_id, video_id, title)` follows the same shape:
look up the playlist (no-op if gone, matching the new "File Deletion
Skipped For a Deleted Playlist" requirement), compute the output directory
and `VideoFilename::from_title(&title)`, call
`video_file_repository.delete(...)`, log the outcome either way.

**`DeleteVideoFileOnVideoDeleted` subscriber only checks `was_downloaded`.**
Unlike `DownloadVideoOnVideoAdded`, it needs no repository lookup — it
doesn't need the playlist for anything the task handler doesn't already
resolve itself. If `was_downloaded`, it schedules
`Task::DeleteVideoFile { playlist_id, video_id, title }` to run now (same
`clock.now()` pattern as the download subscriber); otherwise it's a no-op.

**Task type `delete_video_file`, handler mirrors `DownloadVideoTask`.**
Decodes its payload, no-ops on invalid IDs, delegates to
`VideoService::delete_video_file`. No retry-affecting distinction needed
between "playlist gone" and "file gone" — both are successful no-ops, per
the video-cleanup spec.

## Risks / Trade-offs

- **[Risk] In-flight download outlives the removal that triggered cleanup**
  (see proposal) → accepted; sync's hourly cadence and the planned
  reconcile-style rewrite make the window acceptable rather than solving it
  here.
- **[Risk] Two distinct videos in the same playlist share an identical
  sanitized title and never collided at download time** (e.g. one was
  downloaded, deleted, and its title happens to match another that's still
  present) → deleting by bare-stem match could remove the wrong file, or a
  video that did collide (and thus carries `[video_id]` in its filename) is
  matched unambiguously and this doesn't apply to it. Mitigation: none
  planned — there's no persisted mapping from video ID to actual filename to
  disambiguate further, and this requires two videos to independently
  sanitize to the exact same title, which `resolve_collision` already
  handles correctly (it's designed to prevent overwriting the earlier file's
  content) for concurrently-downloaded videos, only leaves this gap for a
  since-deleted one.
- **[Risk] `DeleteVideoFile` task scheduled for a video whose file download
  is still retrying (`ErroredRetrying`)** → `was_downloaded` is false for
  any non-`Downloaded` status, so no task is scheduled; if that video's
  retry later succeeds after removal, this is the same accepted race as
  above, not a new one.

## Migration Plan

Purely additive: a new `DomainEvent`/`Task` variant, a new subscriber/task
registration, and a new repository port. No schema changes — the `events`
and `tasks` tables already store `event_type`/`task_type` and `payload` as
free-form text/JSON. No rollback concerns beyond a normal revert.
