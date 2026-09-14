## 1. SQLite playlist repository

- [x] 1.1 In `src/infrastructure/repositories/sqlite_playlist_repository.rs`, add `tracing::error!` at every `.context("...")` failure branch in `find`, `insert`, `delete`, `list` (query prepare, execute, and row-read failures — not inside a private helper), and the `new` constructor's table-creation failure, each with `playlist_id`/identifier fields where in scope plus `error = %e`, reusing the existing context string as the message; verify with `cargo build --release` and a manual check that each `.context(` call site now has a matching `.inspect_err(` immediately before it.
- [x] 1.2 Add the same treatment to the `anyhow::anyhow!("database lock poisoned")` branches in this file; verify by inspection that every `.lock()` call site on the connection mutex logs before returning the poison error.

## 2. SQLite video repository

- [x] 2.1 In `src/infrastructure/repositories/sqlite_video_repository.rs`, add `tracing::error!` at every `.context("...")` failure branch in `save`, `find`, `list_for_playlist`, `delete`, `update`, `delete_all_for_playlist` (including the one behind the original bug report), and the `new` constructor, with `playlist_id`/`video_id` fields where in scope plus `error = %e`; verify with `cargo build --release`.
- [x] 2.2 Add the same treatment to this file's `anyhow::anyhow!("database lock poisoned")` branches; verify by inspection.

## 3. SQLite task repository

- [x] 3.1 In `src/infrastructure/repositories/sqlite_task_repository.rs`, add `tracing::error!` at every `.context("...")` failure branch in `schedule` (the `?` before the existing success `info!`), `list_eligible`, `list_running`/`list_non_completed` (via `list_where`), `update`, `delete`, `dead_letter` (all four context strings: begin transaction, insert, delete-after-move, commit), and the `new` constructor's two table-creation calls, with `task_id` where in scope plus `error = %e`; verify with `cargo build --release`.
- [x] 3.2 Add the same treatment to this file's `anyhow::anyhow!("database lock poisoned")` branches; verify by inspection.

## 4. Filesystem video-file repository

- [x] 4.1 In `src/infrastructure/repositories/filesystem_video_file_repository.rs`, add `tracing::error!` immediately before constructing each `anyhow::anyhow!(...)` in `delete`, `list`, and `delete_dir_recursive` (file/directory path, and `error = %e` for the underlying `std::io::Error`), leaving the `NotFound`-is-ok branches untouched; verify with `cargo build --release`.

## 5. YouTube API repositories (key-redaction sensitive)

- [x] 5.1 In `src/infrastructure/repositories/youtube_playlist_repository.rs::exists`, add logging per design.md's redaction rule: the `.context("YouTube API request failed")` `?`-propagation branch logs the `playlist_id` and a fixed message with no `error = %e`; the `anyhow::bail!("...status {status}...")` branch logs `playlist_id` and `status` as fields (no need for `%e` since the message is already known); the `.context("failed to parse YouTube API response")` branch logs `playlist_id` and `error = %e`. Verify with `cargo build --release` and by grepping the diff for `self.api_key` / `%e` co-occurring with the request-failed branch to confirm the key can't reach a log line.
- [x] 5.2 Apply the same three-branch treatment to `src/infrastructure/repositories/youtube_video_repository.rs::find` (`video_id` field); verify with `cargo build --release`.
- [x] 5.3 Apply the same three-branch treatment to `src/infrastructure/repositories/youtube_playlist_items_repository.rs::fetch_page` (`playlist_id` and `page_token` fields where in scope), which is the sole path `list_current_videos` propagates through; verify with `cargo build --release`.

## 6. Verification

- [x] 6.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked` to confirm no regressions and that the `logging` capability's "No Log Output in Automated Tests" requirement still holds.
- [x] 6.2 Manually trigger at least one real failure per repository family (e.g. delete a playlist while the DB file is chmod'd read-only, or point `YARRTUBE_VIDEOS_PATH`/the YouTube API key at something invalid) and confirm a structured `error` log line appears with the expected identifier fields and no API key in it.
