## Why

A playlist delete from the web UI recently failed with "failed to delete
videos for playlist" and no corresponding line in the backend log. Every
fallible method across `src/infrastructure/repositories/` (SQLite repos,
the filesystem video-file repo, the YouTube API repos) propagates errors
purely via `?`/`.context(...)` with no logging at the point of failure —
so any repository-level error is invisible in production until it surfaces
as an opaque message in an HTTP response or a generic, context-free
`error!(error = %e, "poll failed")` one layer up in the two background
orchestrators. This violates the existing `logging` capability's "Failure
is emitted at error level" and "Identifying values are attached as fields"
requirements, which the repository layer doesn't currently meet. We need
this in place before the next deploy so the next occurrence of this bug (or
any other repository failure) is diagnosable from the daemon's logs alone.

## What Changes

- Add a `tracing::error!` (or `warn!` where an existing retry path already
  treats the failure as transient, e.g. task/event retry vs. dead-letter)
  at the point of failure in every fallible method of every repository
  implementation in `src/infrastructure/repositories/`: the three SQLite
  repos (playlist, video, task), the filesystem video-file repo, and the
  three YouTube API repos.
- Attach structured identifying fields already available at the call site
  (e.g. `playlist_id`, `video_id`, `task_id`, `output_dir`) plus
  `error = %e`, following the field-naming idiom already used in
  `task_executor.rs`/`domain_events_consumer.rs` (bare fields for
  owned/`Copy` values, `%` prefix for `Display`-only types, message string
  last), and reuse each existing `.context("...")` string as the log
  message so wording isn't duplicated or drifted.
- Also cover the currently-silent `anyhow::anyhow!("database lock
  poisoned")` branches in the SQLite repos — these are repository-level
  failures today with no visibility either.
- No behavior change beyond the added log lines: no new retries, no
  timeout/locking fix, no change to what any repository method returns to
  its caller. This is pure observability to capture the real error from
  the next occurrence of the delete-playlist bug (or any other repo
  failure) in production.
- Explicitly out of scope: the HTTP handler layer (`http/error.rs` and
  handlers), which still logs nothing when a domain error is converted to
  a response, and the suspected root cause (no `PRAGMA busy_timeout`/WAL
  across the multiple independent SQLite connections). Both are follow-up
  work once the logged error confirms the actual failure mode.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
(none — the existing `logging` capability's requirements already describe
this behavior; this change brings the repository layer into compliance
with them rather than changing what they require)

## Impact

- Code: every file in `src/infrastructure/repositories/` (`sqlite_playlist_repository.rs`,
  `sqlite_video_repository.rs`, `sqlite_task_repository.rs`,
  `filesystem_video_file_repository.rs`, `youtube_playlist_repository.rs`,
  `youtube_playlist_items_repository.rs`, `youtube_video_repository.rs`,
  `youtube_video_downloader_repository.rs`).
- No API, schema, or dependency changes. `tracing`/`tracing-subscriber` are
  already dependencies and already initialized in `serve.rs`.
- Slightly more log volume at `warn`/`error` level in production; no change
  to `info`-level or test-time behavior (tests already suppress log output
  per the `logging` capability's "No Log Output in Automated Tests"
  requirement).
