## Why

The `/api/tasks` endpoint only resolves container context for `reconcile_playlist`/`reconcile_channel` today; every other task type (`download_video`, `delete_video_file`, `delete_playlist_files`, `delete_channel_files`) returns `playlist_name`/`channel_name` as `null`, so the SPA can't say which video is downloading or which playlist/channel a task concerns. The SPA (`TasksView.jsx`) already expects a `video_title` field the API never sends, and has no case at all for `reconcile_channel`, silently falling back to the raw `task_type` string even though the backend already resolves its channel name. Separately, the enrichment that does exist lives directly in the HTTP handler (`list_tasks`), which violates this repo's own layering rule that orchestration belongs in `domain/services/`, not in an adapter.

## What Changes

- Add a `TaskViewSearcher` domain service (`domain/services/`) with `search_all_pending() -> Vec<TaskView>`, replacing the enrichment logic currently inlined in `application/http/tasks/mod.rs::list_tasks`.
- Add a `TaskView` read-model: common fields (`id`, `task_type`, `status`, `retries`, `run_at`, `created_at`, `last_error`) plus `payload: HashMap<String, String>`, whose keys are determined by `task_type`.
- Extend enrichment to every non-completed task type instead of only the two `reconcile_*` types:
  - `reconcile_playlist` -> `{ playlist_name }`
  - `reconcile_channel` -> `{ channel_name }`
  - `download_video` -> `{ video_title, playlist_name }` or `{ video_title, channel_name }`, depending on which container the video belongs to
  - `delete_video_file` -> `{ filename }` (echoed from the payload, no lookup)
  - `delete_playlist_files` -> `{ path }` (echoed from the payload, no lookup)
  - `delete_channel_files` -> `{ path }` (echoed from the payload, no lookup)
  - `update_ytdlp` -> `{}`
- Add `find_by_video(video_id: &VideoRecordId)` to both `PlaylistVideoRepository` and `ChannelVideoRepository`, needed to resolve which container a `download_video` task's video belongs to (its payload carries only the video's surrogate id, not a container id).
- **BREAKING**: `GET /api/tasks`'s response shape changes — the fixed `playlist_name`/`channel_name` fields are replaced by a generic `payload: HashMap<String, String>` object whose keys vary by `task_type`.
- Update `web/src/components/TasksView.jsx`'s `describeTask()` to read from the new `payload` map, add the missing `reconcile_channel` case, and generalize the "in {container}" phrasing to cover a channel as well as a playlist.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `task-listing`: the response contract changes from two fixed, mostly-`null` fields (`playlist_name`/`channel_name`) covering 2 of 7 task types to a per-task-type `payload` map covering all 7, and the requirement is broadened so every non-completed task type gets whatever context is resolvable from its stored payload (a video's title and its owning playlist/channel for `download_video`; the raw filename/path for file-cleanup tasks, which carry no resolvable id).

## Impact

- `src/domain/services/task_view_searcher.rs` (new), registered alongside the other domain services in `serve.rs::build_application`
- `src/domain/task/` (new `TaskView` read-model)
- `src/infrastructure/repositories/sqlite_playlist_video_repository.rs`, `sqlite_channel_video_repository.rs` (new `find_by_video` method, real + fake implementations)
- `src/application/http/tasks/mod.rs`, `dto.rs` (handler thins to one domain call; DTO shape changes)
- `src/serve.rs` (wire `TaskViewSearcher` into `AppState`)
- `web/src/components/TasksView.jsx`, `web/src/api.js` if the response typing needs updating
- `openspec/specs/task-listing/spec.md` (delta)
