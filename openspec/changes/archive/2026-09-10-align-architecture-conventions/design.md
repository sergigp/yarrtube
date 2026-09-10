## Context

See `proposal.md` for motivation. Relevant current state:

- Every repository file under `src/infrastructure/repositories/` is already named `<implementation>_<port>.rs` (`sqlite_playlist_repository.rs`, `youtube_video_downloader_repository.rs`, ...) except `video_file_repository.rs`, whose implementation is `FilesystemVideoFileRepository`.
- `sqlite_event_repository.rs` holds two ports in one file: `EventPublisher` and `EventRepository`.
- `system_clock.rs` (port `Clock`) lives in `repositories/` even though it isn't a per-aggregate port — every domain service injects it.
- `TaskExecutor::poll_once` and the domain-events consumer both do `list_eligible() -> Vec<i64>` then `find(id) -> Option<Entity>` per id, then mutate. Both run as a single `tokio::spawn`'d loop (`serve.rs`), awaited sequentially tick-to-tick; nothing outside the executor's own loop deletes or updates an existing task/event row (only `.schedule()`/`.insert_pending()` add new ones). `recover_stuck_tasks()` runs once at startup, before the loop is spawned, and uses a separate `list_running()` query — it does not interact with `list_eligible`.
- `domain/playlist/service.rs` and `domain/video/service.rs` each carry a `#[cfg(test)] mod tests` exercising the service directly. `http/playlists/mod.rs` already has equivalent-or-superset coverage for the playlist scenarios. `tasks/download_video_task.rs` and `tasks/delete_video_file_task.rs` currently only cover a subset of the video-service scenarios (success path and payload-validation no-ops), missing: errored-retrying vs. terminal-errored, no-op when playlist/video no longer exists, sanitized-filename pass-through, no-op when playlist gone or no matching file found (file deletion).

## Goals / Non-Goals

**Goals:**
- Make the codebase match the rephrased skill exactly for every file this change touches.
- Preserve current runtime behavior and test coverage — this is a structural move, not a behavior change.
- Leave the skill's `shared/` vs `repositories/` line unambiguous for future additions (a new cross-aggregate port should obviously go in `shared/`; a new per-aggregate port obviously in `repositories/`).

**Non-Goals:**
- No change to HTTP/CLI surface, persisted schema, or `yt-dlp` update flow (`cli/ytdlp_update.rs` is explicitly out of scope for this change).
- No introduction of multi-instance/parallel task execution — the `list_eligible` simplification assumes the current single-poller topology and is not a defense against future horizontal scaling. Revisit if that changes.
- Not attempting to reconcile VO-level domain tests (`playlist_name.rs`, `quality.rs`, etc.) against application-layer reachability — left as-is per explicit decision during exploration.

## Decisions

**`shared/` redefinition.** Old rule: "infra with no trait/port boundary at all." New rule: infra usable across aggregates, regardless of whether it's a port. Rationale: `Clock` and the domain-event ports are both ports, both injected into multiple aggregate services, and neither belongs to one aggregate the way `PlaylistRepository` belongs to `playlist`. Keeping them in `repositories/` (defined as "port implementations a domain service injects") was technically consistent with the old wording but grouped a cross-cutting concern next to per-aggregate ports, which is the actual source of the mismatch. `client/` is unaffected by this change (still: "port-shaped adapters nothing in `domain/` injects").

**File naming: `<implementation>_<port>.rs`, codified rather than changed.** The codebase already does this everywhere but one file; codifying it in the skill is cheaper and less disruptive than moving to a bare `<port>.rs` naming scheme, and the explicit prefix is useful signal (which technology backs this port) that the team wants to keep.

**Split `EventPublisher`/`EventRepository` into two files.** One port per file, matching the "every value object gets its own file" spirit already applied elsewhere — `sqlite_event_repository.rs` was the only repository file bundling two port traits.

**Collapse `list_eligible` + `find` into one entity-returning read.** The two-phase read exists to protect against a row changing between listing and dispatch. Traced every writer of `task`/`event` rows: only the executor's own loop deletes/updates existing rows (everything else only inserts). The loop is strictly sequential (each `poll_once` fully awaited before the next tick), and startup recovery runs before the loop starts. So the guard defends against a scenario with no current trigger. Simplify to a single query returning `Vec<Task>` / `Vec<ScheduledEvent>` (dropping the ID-only intermediate) and delete the "disappeared before dispatch" branch. Alternative considered: keep the two-phase read and just document it as a deliberate deviation — rejected because it adds a permanent exception for a scenario that isn't live, when "keep it simple until needed" was the explicit preference.

**Delete domain-service tests only after backfilling task-level coverage.** Deleting `domain/video/service.rs`'s tests first would lose coverage for retry/terminal-error branching, sanitized-filename pass-through, and the playlist/video/file no-op paths, since `tasks/download_video_task.rs` and `tasks/delete_video_file_task.rs` don't yet assert them. Order matters: add the missing task-level tests first, confirm they pass, then delete the domain-level module. `domain/playlist/service.rs` has no such gap (`http/playlists/mod.rs` already covers every scenario), so its test module can be deleted directly.

## Risks / Trade-offs

- [Renaming/moving files breaks intermediate compilation state if done as one large edit] → Land the rename/move and the `list_eligible` simplification as separate, independently-buildable steps (see tasks.md ordering); run `cargo build`/`cargo test` after each.
- [Deleting domain-level tests before task-level replacements exist would silently drop coverage] → Enforced by task ordering: add-then-verify-then-delete, never delete-then-add.
- [Removing the `list_eligible`/`find` staleness guard removes a safety net that could matter if task execution is ever parallelized] → Accepted per explicit product decision; call it out again if/when multiple executor instances are considered.

## Migration Plan

No runtime migration — this is a compile-time restructuring of internal modules plus a doc update. No data, schema, or deployed-artifact changes. Standard PR + CI (fmt, clippy, build, test) is the rollout; revert is a plain `git revert` if needed.
