## 1. Task querying (domain + infrastructure)

- [x] 1.1 Add `TaskRepository::list_non_completed(&self) -> anyhow::Result<Vec<ScheduledTask>>` to the trait in `src/infrastructure/repositories/sqlite_task_repository.rs`, implement it in `SqliteTaskRepository` (`SELECT ... WHERE status IN ('pending', 'running')`, no `run_at` filter) and in `FakeTaskRepository`, and verify with an infrastructure test asserting a `pending` task with a future `run_at` is included and a task with any other status is not.
- [x] 1.2 Add `src/domain/task/service.rs` with `TaskService` (holds `Arc<dyn TaskRepository>`) and `list_non_completed(&self) -> anyhow::Result<Vec<ScheduledTask>>`, and export it from `src/domain/task/mod.rs`.

## 2. Video querying (domain)

- [x] 2.1 Add `ListVideosError` to `src/domain/video/errors.rs` (at least a `PlaylistNotFound` variant) and add `VideoService::list_videos(&self, playlist_id: &PlaylistId) -> Result<Vec<Video>, ListVideosError>` to `src/domain/video/service.rs`, checking playlist existence via the existing `playlist_repository` before delegating to `video_repository.list_for_playlist`. Cover with behavior tests: existing playlist with videos, existing playlist with none, nonexistent playlist.

## 3. HTTP: new endpoints

- [x] 3.1 Add `http/videos/{mod.rs,dto.rs}`: a `VideoResponse` DTO (id, title, status, quality, filename, created_at, updated_at) and a `list_videos_for_playlist` handler mapping `ListVideosError::PlaylistNotFound` to `400`. Wire `GET /playlists/{id}/videos` in the router (still unprefixed at this point — prefixing happens in task 4). Cover with the same three scenarios as 2.1, asserted through the router as in the existing `playlists`/`custom_playlists` handler tests.
- [x] 3.2 Add `http/tasks/{mod.rs,dto.rs}`: a `TaskResponse` DTO (id, task_type, status, retries, run_at, created_at, last_error) and a `list_tasks` handler calling `TaskService::list_non_completed`. Wire `GET /tasks` in the router. Cover with behavior tests: tasks exist, none exist, a future-`run_at` pending task is still included.
- [x] 3.3 Add `task_service: TaskService` to `AppState` (`src/http/mod.rs`) and wire it in `build_application()` (`src/serve.rs`).

## 4. HTTP: route layout

- [x] 4.1 In `src/serve.rs::serve_http`, nest the existing JSON router (playlists, custom-playlists, the new videos/tasks endpoints) under `/api`, leaving `GET /status` unprefixed. Update every router test's request URIs (`src/http/playlists/mod.rs`, `src/http/custom_playlists/mod.rs`, and the new test modules from step 3) to the `/api/...` paths and verify `cargo test` passes.
- [x] 4.2 Update every `curl` example and the endpoint table in `README.md` to the new `/api/...` paths; verify by re-reading the README for any remaining unprefixed path.

## 5. Frontend scaffold

- [x] 5.1 Scaffold a Vite + React project under `web/` (`npm create vite@latest`, React + JS or TS per your preference), verify `npm run dev` serves a blank page and `npm run build` produces `web/dist/`.
- [x] 5.2 Build the playlists list view: fetches `GET /api/playlists`, renders each with a click handler that switches to the detail view; verify manually against a running `serve` instance with at least one tracked playlist.
- [x] 5.3 Build the playlist detail view: fetches `GET /api/playlists/{id}/videos` on an interval (3s, per design.md), renders the video list with status, and a back control returning to the playlist list; verify a video's displayed status updates within one poll cycle after its status changes in the database.
- [x] 5.4 Build the tasks tab: fetches `GET /api/tasks` on the same 3s interval, renders each task's type/status/retries; verify a newly scheduled task appears and a completed one disappears within one poll cycle.
- [x] 5.5 Add top-level navigation between the playlists view and the tasks tab; verify all three views (list, detail, tasks) are reachable and the detail view's back control returns to the list.

## 6. Embedding and serving the SPA

- [x] 6.1 Add the `rust-embed` dependency to `Cargo.toml`, add a `#[derive(RustEmbed)] #[folder = "web/dist/"]` asset type (e.g. in a new `src/infrastructure/shared/web_assets.rs`), and add `.gitignore` entries for `web/dist/` and `web/node_modules/`.
- [x] 6.2 Add a fallback handler in `src/serve.rs` (or a new `http` module) serving `/` and any unmatched path from the embedded assets (`index.html` for `/`, the matching embedded file for anything else, `404` otherwise), mounted after the `/api` and `/status` routes. Verify with a router test: `GET /` returns the embedded HTML, and a known asset path returns its content.

## 7. Build pipeline (Docker, CI, local dev)

- [x] 7.1 Add a Node build stage to `Dockerfile` (`npm ci && npm run build` in `web/`) ahead of the existing Rust build stage, and copy `web/dist/` into the Rust build context before `cargo build --release`. Verify with a local `docker build .` that succeeds and that the resulting container serves `/` with the built UI.
- [x] 7.2 Add an `npm ci && npm run build` step to `.github/workflows/ci.yml` before the existing fmt/clippy/build/test steps. Verify by pushing to a branch and confirming CI passes.
- [x] 7.3 Add the one-time `cd web && npm ci && npm run build` step to `DEVELOPMENT.md`'s local setup section, ahead of `cargo build --release`.

## 8. Final verification

- [x] 8.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`; all pass.
- [x] 8.2 Run `./target/release/yarrtube serve` locally against a populated database and manually exercise the full flow: playlist list → click a playlist → videos list with live status updates → back → tasks tab with live pending/in-progress tasks.
