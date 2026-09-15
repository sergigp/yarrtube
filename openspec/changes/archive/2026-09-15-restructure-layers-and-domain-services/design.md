## Context

See proposal.md - Why/What Changes for motivation and the full list of moves. This document covers the technical shape of each move: exact new types/files, their dependencies, and how the `application/` split and the `cli`/`infrastructure` fix fit together. Findings below (dependency lists, caller counts) come from tracing actual field usage and call sites in the current codebase, not from the method signatures alone.

## Goals / Non-Goals

**Goals:**
- Every new domain service is injected with only the ports its own methods call — no port a service holds but never uses.
- The physical file tree matches the three layers the skill already describes conceptually (domain / application / infrastructure), and `domain/services/` files map 1:1 to a single use case each.
- Fn ordering (constructor, then public/trait-impl methods, then private helpers) is uniform across the codebase, with no carve-out for repositories.

**Non-Goals:**
- No behavior change. No new capability, no changed HTTP contract, no changed task/event payload shapes.
- No introduction of a DI framework or a new abstraction to hold groups of the narrower services together (e.g. no `PlaylistServices` bundle struct) — `AppState` and every task/subscriber keep holding exactly the flat set of narrow services they need, consistent with how `AppState` already holds `playlist_service`/`video_service`/`channel_service`/`task_service` today.
- No change to the domain event / task-scheduling plumbing itself (`DomainEventsConsumer`, `TaskExecutor`, the registries) — only the handlers/subscribers they call are updated to depend on narrower services.

## Decisions

### 1. Fn ordering — no repository exception

Order inside every `impl` block, and among free functions in a file: `new` (if present) → every `pub` method or trait-impl method (trait-impl methods are the type's public surface even without the `pub` keyword) → private/helper methods, in that order. This applies uniformly, including to the three SQLite repositories, whose `row_to_*` mapping helper currently sits between `new` and the trait-impl methods — it moves to the end, after `list`/`delete`/etc.

Files needing reordering (confirmed by inspection, not the split-affected files below, which get this for free as a byproduct of the rewrite):
- `infrastructure/repositories/sqlite_playlist_repository.rs` (`row_to_playlist`)
- `infrastructure/repositories/sqlite_video_repository.rs` (`row_to_video`)
- `infrastructure/repositories/sqlite_channel_repository.rs` (`row_to_channel`)
- `infrastructure/repositories/filesystem_video_file_repository.rs` (`is_in_progress_temp_file`)
- `infrastructure/shared/domain_events/event_publisher.rs` (`insert_pending_row`)

`infrastructure/shared/ytdlp.rs` (`resolve_collision`) and `domain/video/video_filename.rs` (`is_keepable`, `truncate_at_char_boundary`) already comply — no change needed.

### 2. `domain/services/` — flat, one file per use case

```
domain/
  playlist/            <- entities, value objects, errors.rs (unchanged)
  video/                <- entities, value objects, errors.rs (unchanged)
  services/             <- NEW: every aggregate's use-case services, together
    playlist_creator.rs
    playlist_deleter.rs
    playlist_searcher.rs
    video_reconciler.rs
    video_downloader.rs
    video_file_deleter.rs
    custom_playlist_video_adder.rs
    custom_playlist_video_remover.rs
    video_searcher.rs
```

Chosen over nesting under each aggregate (`domain/playlist/services/...`) per explicit preference: one folder to scan every use case regardless of aggregate. Value objects/entities/`errors.rs` stay put — only orchestration moves out.

### 3. Playlist split

| Type | Fields | Methods | Drops (vs. old `PlaylistService`) |
|---|---|---|---|
| `PlaylistCreator` | `repository`, `lookup`, `event_publisher`, `clock` | `create`, `create_custom` | `video_repository` |
| `PlaylistDeleter` | `repository`, `video_repository`, `event_publisher` | `delete` | `lookup`, `clock` |
| `PlaylistSearcher` | `repository` | `search`, `search_all` | `video_repository`, `lookup`, `event_publisher`, `clock` |

`path_used_by_another_playlist` (private helper used by both `create` and `create_custom`) moves into `playlist_creator.rs`, at the end of the file per decision 1.

### 4. Video split

| Type | Fields | Methods (renamed) | Notes |
|---|---|---|---|
| `VideoReconciler` | `playlist_repository`, `video_repository`, `youtube_playlist_items_repository`, `event_publisher`, `task_repository`, `video_file_repository`, `clock`, `reconcile_interval_seconds`, `videos_path` | `reconcile`, `force_reconcile` | Private: `run_reconcile_pass`, `sync_playlist_membership`, `reconcile_filesystem` — `reconcile_filesystem` is `pub` today but has zero external callers, so it becomes private here. |
| `VideoDownloader` | `playlist_repository`, `video_repository`, `video_downloader_repository`, `clock`, `videos_path` | `download` | Name mirrors the infra port `VideoDownloaderRepository`; accepted, disambiguated by module path (`domain::services::VideoDownloader` vs `infrastructure::repositories::youtube_video_downloader_repository::VideoDownloaderRepository`). |
| `VideoFileDeleter` | `playlist_repository`, `video_file_repository`, `videos_path` | `delete_video_file`, `delete_playlist_video_files` (renamed from `delete_playlist_files`) | Both methods delete, so both keep a distinguishing noun; stays with video (not moved to the playlist aggregate) since it's grouped by the `VideoFileRepository` port it shares, and the rename makes clear it's about video files, not playlist metadata. |
| `CustomPlaylistVideoAdder` | `playlist_repository`, `video_repository`, `youtube_video_repository`, `event_publisher`, `clock` | `add` (renamed from `add_video_to_custom_playlist`) | |
| `CustomPlaylistVideoRemover` | `playlist_repository`, `video_repository`, `event_publisher` | `remove` (renamed from `remove_video_from_playlist`) | Drops `youtube_video_repository`, `clock` vs. Adder. |
| `VideoSearcher` | `playlist_repository`, `video_repository` | `find`, `list` (renamed from `list_videos`) | Down from 11 fields on the old `VideoService` to 2. |

Every public method today has exactly one external caller site (confirmed by grep), which is what makes this granularity safe: each split type maps onto a real, singular usage pattern rather than a guess.

### 5. `application/` top-level layer

```
src/
  domain/                 (unchanged, plus domain/services/)
  application/            <- NEW
    http/                 (moved as-is)
    subscribers/          (moved as-is)
    tasks/                (moved as-is)
    cli/
      mod.rs               <- Cli/Commands (clap parsing), moved from src/cli/mod.rs
      update_ytdlp.rs      <- thin dispatcher: calls the YtdlpUpdater port, prints result, returns ExitCode
  infrastructure/          (unchanged, plus the ytdlp_updater.rs merge)
main.rs                    <- `mod application;` replacing `mod http; mod subscribers; mod tasks; mod cli;`
```

This isn't a new layer conceptually — the skill's Application Layer section already names HTTP controllers, CLI, and subscribers as the same kind of thing (entry point, validate + transform, call domain, map result back). The physical tree just didn't reflect that; this change makes it explicit. The skill's "Deliberate deviations" note ("Orchestration lives in `domain/<aggregate>/service.rs`, not a separate `application/` layer") is about where *business orchestration* lives, not about adapters — it remains true after this change (orchestration is now in `domain/services/`, still domain, never `application/`). The skill update adds a one-line clarification next to that deviation so a reader doesn't misread the new `application/` folder as contradicting it.

### 6. `cli/` split by responsibility, fixing the dependency inversion

Today `infrastructure/client/ytdlp_updater.rs`'s `RealYtdlpUpdater` (the actual port implementation) delegates to `crate::cli::ytdlp_update::update(...)` — infrastructure depending on the CLI/application layer, backwards from every other port in the codebase.

Fix: merge `latest_linux_binary_url`, `download_bytes`, `install_binary`, `update`, and `target_path` from `cli/ytdlp_update.rs` directly into `infrastructure/client/ytdlp_updater.rs`. `RealYtdlpUpdater::update` calls this logic directly instead of reaching into `cli::`. `application/cli/update_ytdlp.rs` keeps only `run()` (parses nothing extra, calls the `YtdlpUpdater` port the same way `UpdateYtdlpTask` already does, prints, returns `ExitCode`) — this makes the CLI dispatcher symmetric with an HTTP handler: thin, no logic, just calls a port/service and maps the result.

## Risks / Trade-offs

- **Large mechanical diff, low logical risk**: this touches ~30 files but almost every change is "move this code, update these `use` paths, thread a narrower type through a constructor" — no behavior changes. Mitigate by relying on the existing test suite (behavior tests move with their code and must still pass unmodified in assertions) and `cargo build`/`cargo clippy` catching every broken import immediately.
- **`AppState` grows more fields** (2 fat services become up to 9 narrow ones, plus `channel_service`/`task_service`). Accepted per Non-Goals — no bundling abstraction — but flag if `AppState`'s field count becomes unwieldy in practice; that's a future call, not blocking this change.
- **Six new files for one aggregate (video)** vs. three for playlist may look asymmetric at a glance. This is intentional — it reflects six genuinely distinct usage patterns (six different single-caller-site methods), not six behaviors invented for symmetry.

## Migration Plan

Single-PR, no runtime migration (no data/schema change, no deployed-behavior change). Suggested order (mirrors tasks.md): skill update first (so the rest of the work has a written target to match) → fn-ordering fixes in already-existing files → `domain/services/` split (playlist, then video) and all call-site updates → `application/` move → `cli`/`infrastructure` fix → final `cargo fmt`/`clippy`/`test` pass. No rollback concerns beyond reverting the commit(s); no feature flag needed since nothing is user-visible.
