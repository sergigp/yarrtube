## 1. Skill update

- [x] 1.1 Rewrite the File Structure section of `.claude/skills/rust-architect/SKILL.md` to show `domain/services/` (one file per use case) and the new `application/{http,cli,subscribers,tasks}` layout; verify by re-reading the section.
- [x] 1.2 Add the fn-ordering rule to the skill (`new` if present → every `pub`/trait-impl method → private/helper methods, no exception for repositories' row-mapping helpers); verify the rule text is present and unambiguous.
- [x] 1.3 Add a line stating domain services are call-agnostic and that adapting any external trigger (HTTP, CLI, subscriber, task) into a domain call is the application layer's sole responsibility; add a one-line clarification next to the existing "Deliberate deviations" entry about orchestration not living in a separate application layer, so the new `application/` folder isn't misread as contradicting it.

## 2. Fn-ordering fixes (files untouched by the splits below)

- [x] 2.1 `infrastructure/repositories/sqlite_playlist_repository.rs`: move `row_to_playlist` to the end, after the trait-impl methods; `cargo build` succeeds.
- [x] 2.2 `infrastructure/repositories/sqlite_video_repository.rs`: move `row_to_video` to the end; `cargo build` succeeds.
- [x] 2.3 `infrastructure/repositories/sqlite_channel_repository.rs`: move `row_to_channel` to the end; `cargo build` succeeds.
- [x] 2.4 `infrastructure/repositories/filesystem_video_file_repository.rs`: move `is_in_progress_temp_file` to the end; `cargo build` succeeds.
- [x] 2.5 `infrastructure/shared/domain_events/event_publisher.rs`: move `insert_pending_row` to the end; `cargo build` succeeds.

## 3. Split playlist domain service

- [x] 3.1 Create `domain/services/playlist_creator.rs`: `PlaylistCreator { repository, lookup, event_publisher, clock }` with `create`/`create_custom`, and the private `path_used_by_another_playlist` helper at the end of the file; move its existing unit tests; `cargo test playlist_creator` passes.
- [x] 3.2 Create `domain/services/playlist_deleter.rs`: `PlaylistDeleter { repository, video_repository, event_publisher }` with `delete`; move its existing unit tests; `cargo test playlist_deleter` passes.
- [x] 3.3 Create `domain/services/playlist_searcher.rs`: `PlaylistSearcher { repository }` with `search`/`search_all`; move its existing unit tests; `cargo test playlist_searcher` passes.
- [x] 3.4 Delete `domain/playlist/service.rs`; add `pub mod services;` to `domain/mod.rs` and a `domain/services/mod.rs` re-exporting the three new types; `cargo build` succeeds.
- [x] 3.5 Update `serve.rs`'s `build_application` to construct `PlaylistCreator`/`PlaylistDeleter`/`PlaylistSearcher` in place of `PlaylistService::new(...)` (around the current playlist-service construction).
- [x] 3.6 Update `http/playlists/mod.rs`: `create_playlist` uses `PlaylistCreator`, `delete_playlist` uses `PlaylistDeleter`, `list_playlists` uses `PlaylistSearcher`; update `AppState` fields and the module's test-fixture constructions (there are three in this file) accordingly.
- [x] 3.7 Update `http/custom_playlists/mod.rs`: `create_custom_playlist` uses `PlaylistCreator`; update its test-fixture construction.
- [x] 3.8 Update `http/tasks/mod.rs`: `find_playlist` lookup uses `PlaylistSearcher`; update its test-fixture construction.
- [x] 3.9 Update the `PlaylistService` test-fixture constructions in `http/videos/mod.rs` and `http/channels/mod.rs` to build only the narrow service(s) those tests actually exercise (likely `PlaylistSearcher`, confirm per test).
- [x] 3.10 `cargo test` passes with every playlist-related behavior test's assertions unchanged.

## 4. Split video domain service

- [x] 4.1 Create `domain/services/video_reconciler.rs`: `VideoReconciler { playlist_repository, video_repository, youtube_playlist_items_repository, event_publisher, task_repository, video_file_repository, clock, reconcile_interval_seconds, videos_path }` with `reconcile`/`force_reconcile`, and private `run_reconcile_pass`/`sync_playlist_membership`/`reconcile_filesystem` at the end; move its existing unit tests; `cargo test video_reconciler` passes.
- [x] 4.2 Create `domain/services/video_downloader.rs`: `VideoDownloader { playlist_repository, video_repository, video_downloader_repository, clock, videos_path }` with `download`; move its existing unit tests; `cargo test video_downloader` passes.
- [x] 4.3 Create `domain/services/video_file_deleter.rs`: `VideoFileDeleter { playlist_repository, video_file_repository, videos_path }` with `delete_video_file`/`delete_playlist_video_files` (renamed from `delete_playlist_files`); move its existing unit tests; `cargo test video_file_deleter` passes.
- [x] 4.4 Create `domain/services/custom_playlist_video_adder.rs`: `CustomPlaylistVideoAdder { playlist_repository, video_repository, youtube_video_repository, event_publisher, clock }` with `add` (renamed from `add_video_to_custom_playlist`); move its existing unit tests; `cargo test custom_playlist_video_adder` passes.
- [x] 4.5 Create `domain/services/custom_playlist_video_remover.rs`: `CustomPlaylistVideoRemover { playlist_repository, video_repository, event_publisher }` with `remove` (renamed from `remove_video_from_playlist`); move its existing unit tests; `cargo test custom_playlist_video_remover` passes.
- [x] 4.6 Create `domain/services/video_searcher.rs`: `VideoSearcher { playlist_repository, video_repository }` with `find`/`list` (renamed from `list_videos`); move its existing unit tests; `cargo test video_searcher` passes.
- [x] 4.7 Delete `domain/video/service.rs`; register the six new modules in `domain/services/mod.rs`; `cargo build` succeeds.
- [x] 4.8 Update `serve.rs`'s `build_application` to construct all six new services in place of the single `VideoService::new(...)` call.
- [x] 4.9 Update `subscribers/reconcile_on_playlist_created.rs` to depend on `VideoReconciler` instead of `VideoService`; update its test-fixture construction.
- [x] 4.10 Update `tasks/reconcile_playlist_task.rs` to depend on `VideoReconciler`; update its test-fixture constructions (two in this file).
- [x] 4.11 Update `tasks/download_video_task.rs` to depend on `VideoDownloader`; update its test-fixture constructions (three in this file).
- [x] 4.12 Update `tasks/delete_video_file_task.rs` to depend on `VideoFileDeleter`; update its test-fixture construction.
- [x] 4.13 Update `tasks/delete_playlist_files_task.rs` to depend on `VideoFileDeleter` and call `delete_playlist_video_files`; update its test-fixture construction.
- [x] 4.14 Update `http/videos/mod.rs`'s `list_videos_for_playlist` handler to depend on `VideoSearcher::list`; update `AppState` and its test-fixture construction.
- [x] 4.15 Update `http/custom_playlists/mod.rs`'s `add_video`/`remove_video` handlers to depend on `CustomPlaylistVideoAdder::add`/`CustomPlaylistVideoRemover::remove`; update `AppState` and its test-fixture construction.
- [x] 4.16 Update `http/tasks/mod.rs`'s `build_task_response` to depend on `VideoSearcher::find`; update `AppState` and its test-fixture construction.
- [x] 4.17 Update `tasks/mod.rs`'s `registry(...)` wiring so each `TaskHandler` receives only the narrow service(s) it needs instead of a shared `VideoService`.
- [x] 4.18 Update `subscribers/mod.rs`'s `registry(...)` wiring the same way.
- [x] 4.19 `cargo test` passes with every video-related behavior test's assertions unchanged.

## 5. Move to `application/` layer

- [x] 5.1 Create `application/mod.rs`; `git mv src/http src/application/http`; update every `crate::http::` reference to `crate::application::http::`; `cargo build` succeeds.
- [x] 5.2 `git mv src/subscribers src/application/subscribers`; update every `crate::subscribers::` reference to `crate::application::subscribers::`; `cargo build` succeeds.
- [x] 5.3 `git mv src/tasks src/application/tasks`; update every `crate::tasks::` reference to `crate::application::tasks::`; `cargo build` succeeds.
- [x] 5.4 Update `main.rs` (`mod application;` replacing `mod http; mod subscribers; mod tasks;`) and `serve.rs`'s `use` statements; `cargo build` succeeds.
- [x] 5.5 `grep -rn "crate::http::\|crate::subscribers::\|crate::tasks::" src` returns nothing outside `application/`; confirms no stale path was missed.

## 6. Split `cli/` and fix the infrastructure dependency inversion

- [x] 6.1 Create `application/cli/mod.rs` with `Cli`/`Commands`, moved from `cli/mod.rs`.
- [x] 6.2 Merge `latest_linux_binary_url`, `download_bytes`, `install_binary`, `update`, and `target_path` from `cli/ytdlp_update.rs` into `infrastructure/client/ytdlp_updater.rs`, applying the fn-ordering rule from section 1; move the existing `it_should_leave_the_original_binary_untouched_when_the_write_fails` test with them; `RealYtdlpUpdater::update` calls this logic directly instead of `crate::cli::ytdlp_update::update`.
- [x] 6.3 Create `application/cli/update_ytdlp.rs` with just `run()`, calling the injected `YtdlpUpdater` port (same pattern `UpdateYtdlpTask` already uses) instead of a free function; returns `ExitCode` as today.
- [x] 6.4 Update `main.rs` to use `application::cli::{Cli, Commands}` and `application::cli::update_ytdlp::run()`; delete the `cli/` directory entirely.
- [x] 6.5 Update `serve.rs`'s `run_startup_ytdlp_update` to build a `RealYtdlpUpdater` and call `.update(&target_path)` through the `YtdlpUpdater` trait instead of calling `ytdlp_update::update`/`ytdlp_update::target_path` directly; update the two other `ytdlp_update::target_path()` call sites (video-downloader construction, `UpdateYtdlpTask` wiring) to call the relocated `target_path()` on `infrastructure::client::ytdlp_updater`.
- [x] 6.6 `grep -rn "crate::cli::" src` returns nothing; confirms the old module and every reference to it are gone.

## 7. Final verification

- [x] 7.1 `cargo fmt --all -- --check` passes.
- [x] 7.2 `cargo clippy --all-targets --all-features --locked -- -D warnings` passes with zero warnings.
- [x] 7.3 `cargo build --release` succeeds.
- [x] 7.4 `cargo test --locked` passes, with the total test count unchanged or higher than the pre-change baseline (no test silently dropped during a move).
