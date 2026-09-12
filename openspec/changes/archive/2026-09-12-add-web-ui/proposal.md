## Why

Today the only way to see what yarrtube is tracking — which playlists, which videos, whether a download is stuck or in progress — is to query the SQLite database or the JSON API by hand. There's no way to glance at the system's state in a browser. This change adds a small browser UI for browsing playlists, drilling into a playlist's videos, and watching pending/in-progress tasks live, without needing a DB client or `curl`.

## What Changes

- **BREAKING**: All existing JSON endpoints move under an `/api/` prefix (`/playlists` → `/api/playlists`, `/custom-playlists` → `/api/custom-playlists`, etc.) so they no longer collide with the new browser-facing routes. Behavior of each endpoint is unchanged, only the path. `GET /status` is unaffected (stays a bare health check).
- Add `GET /api/playlists/{id}/videos` — lists the videos belonging to a playlist.
- Add `GET /api/tasks` — lists tasks that are not yet complete (`pending` or `running`, regardless of scheduled run time), for the tasks tab.
- Add a single-page React application, built with Vite and embedded into the `yarrtube` binary at compile time (no Node/npm at runtime), served at `/` by the `serve` daemon. It shows:
  - A list of tracked playlists.
  - A playlist's videos (and a way back to the list) when a playlist is clicked.
  - A tab listing pending and in-progress tasks.
  - The playlists and tasks views poll their respective endpoints on an interval so state changes (a task starting, a video finishing) show up without a manual refresh.

## Capabilities

### New Capabilities
- `web-ui`: Serves the embedded single-page application from the daemon's HTTP server, covering both the root page and its static assets.
- `video-listing`: HTTP endpoint to list the videos belonging to a playlist.
- `task-listing`: HTTP endpoint to list tasks that have not yet completed.

### Modified Capabilities
_None._ `playlist-crud` and `custom-playlist-crud` describe endpoint behavior, not exact paths — relocating them under `/api/` is an implementation/routing detail, not a requirement-level change, so no delta spec is needed for either.

## Impact

- `src/serve.rs`: router gains an `/api` nest, a static/embedded-asset route for the SPA, and (unchanged) `/status`.
- `src/http/`: existing `playlists`/`custom_playlists` routers move under `/api`; new `http/videos/` and `http/tasks/` modules (handlers + DTOs) are added.
- `src/domain/video/service.rs`: gains a read-only "list videos for playlist" query method.
- `src/domain/task/`: gains a `TaskService` (currently there is none — HTTP has never needed to query tasks) wrapping a new `TaskRepository::list_non_completed()` method.
- `src/infrastructure/repositories/sqlite_task_repository.rs`: new repository method + SQL query.
- New `web/` (or similar) directory: a Vite + React project, `npm`-built.
- `Dockerfile`: gains a Node-based build stage (discarded after `npm run build`); the runtime stage is unaffected (still just the Rust binary).
- `Cargo.toml`: new dependency, `rust-embed`, to bake the built SPA assets into the binary.
- `README.md`: every documented `curl` example updates to the new `/api/...` paths.
