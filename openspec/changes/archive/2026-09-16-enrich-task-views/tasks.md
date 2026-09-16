## 1. Repository ports: reverse video lookup

- [x] 1.1 Add `find_by_video(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<PlaylistVideo>>` to `PlaylistVideoRepository` (`src/infrastructure/repositories/sqlite_playlist_video_repository.rs`), implement it on `SqlitePlaylistVideoRepository` (`WHERE video_id = ?`) and on the fake, and verify with a unit test covering a hit and a miss
- [x] 1.2 Add the equivalent `find_by_video` to `ChannelVideoRepository` (`src/infrastructure/repositories/sqlite_channel_video_repository.rs`), implement on `SqliteChannelVideoRepository` and the fake, and verify with a unit test covering a hit and a miss

## 2. Domain: `TaskView` and `TaskViewSearcher`

- [x] 2.1 Add `TaskView` (`id`, `task_type`, `status`, `retries`, `run_at`, `created_at`, `last_error`, `payload: HashMap<String, String>`) under `src/domain/task/`
- [x] 2.2 Add `TaskViewSearcher` (`src/domain/services/task_view_searcher.rs`) taking `TaskRepository`, `PlaylistRepository`, `ChannelRepository`, `VideoRepository`, `PlaylistVideoRepository`, `ChannelVideoRepository`, with `search_all_pending() -> anyhow::Result<Vec<TaskView>>`
- [x] 2.3 Implement per-`task_type` payload resolution in `TaskViewSearcher` (`reconcile_playlist` -> `playlist_name`; `reconcile_channel` -> `channel_name`; `download_video` -> `video_title` + `playlist_name`/`channel_name` via the new `find_by_video` methods; `delete_video_file` -> `filename`; `delete_playlist_files`/`delete_channel_files` -> `path`; `update_ytdlp` -> empty), verified with one unit test per `task_type` asserting its exact resolved keys
- [x] 2.4 Add unit tests for the "referenced container/video no longer exists" case (omitted key, not an error) and for `download_video` resolving a channel-owned video as well as a playlist-owned one

## 3. Wiring

- [x] 3.1 Register `TaskViewSearcher` in `serve.rs::build_application` and add it to `AppState`, and verify the binary still builds and boots (`cargo build --release`, `./target/release/yarrtube serve` starts cleanly)

## 4. HTTP layer

- [x] 4.1 Replace `TaskResponse`'s `playlist_name`/`channel_name` fields with `payload: HashMap<String, String>` in `src/application/http/tasks/dto.rs`, built from `TaskView`
- [x] 4.2 Rewrite `list_tasks` (`src/application/http/tasks/mod.rs`) to call `task_view_searcher.search_all_pending()` once and map straight to `TaskResponse`, removing the inlined `playlist_searcher`/`channel_service` calls
- [x] 4.3 Update `src/application/http/tasks/mod.rs`'s existing tests for the new response shape, and add coverage for `download_video`, `delete_video_file`, `delete_playlist_files`, and `delete_channel_files` each returning their expected `payload` keys end-to-end through the router
- [x] 4.4 Run `cargo test --locked`, `cargo fmt --all -- --check`, and `cargo clippy --all-targets --all-features --locked -- -D warnings` and confirm all pass

## 5. Frontend

- [x] 5.1 Update `web/src/components/TasksView.jsx`'s `describeTask()` to read `task.payload.video_title`/`task.payload.playlist_name`/`task.payload.channel_name`/`task.payload.filename`/`task.payload.path` instead of the old top-level fields
- [x] 5.2 Add the missing `reconcile_channel` case (mirroring `reconcile_playlist`) and generalize `download_video`'s wording to say "in {channel}" when the container is a channel rather than always assuming a playlist
- [x] 5.3 Add a `default` case fallback that still degrades gracefully for `delete_playlist_files`/`delete_channel_files`/`update_ytdlp`, verified by seeding every `task_type` into a running `serve` instance's SQLite DB, confirming `/api/tasks`'s resolved `payload` matches design.md's key table, and running `describeTask()` against those real payloads to confirm every `task_type` renders a sensible description
