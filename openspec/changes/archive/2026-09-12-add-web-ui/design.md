## Context

See `proposal.md` - Why for motivation. Relevant current state:

- `src/serve.rs::serve_http` builds one flat `axum::Router`, merging `GET /status` with `http::playlists_router`. `AppState` (in `src/http/mod.rs`) currently carries only `playlist_service` and `video_service`.
- `src/domain/task/` has entities/value objects (`Task`, `ScheduledTask`) and a repository port (`TaskRepository` in `src/infrastructure/repositories/sqlite_task_repository.rs`), but no `service.rs` — nothing outside the background task executor has ever needed to query tasks.
- `TaskRepository` only exposes `list_eligible()` (due now) and `list_running()`; neither returns a `pending` task whose `run_at` is still in the future, which a "pending tasks" tab needs.
- The Dockerfile and CI (`.github/workflows/ci.yml`) have no Node/npm step today; `cargo build`/`cargo test` are the only build commands.
- Per `.claude/skills/rust-architect/SKILL.md`, HTTP handlers stay thin adapters calling one domain method; orchestration lives in `domain/<aggregate>/service.rs`; repositories return entities/collections directly, never a wrapper type.

## Goals / Non-Goals

**Goals:**
- Browse playlists, a playlist's videos, and non-completed tasks from a browser, with the tasks/videos views reflecting state changes without a manual reload.
- Keep the deployment story a single binary / single-stage runtime image, matching the project's existing NAS-friendly Docker setup.

**Non-Goals:**
- Playing video files in the browser (explicitly future scope; needs a streaming route that doesn't exist yet).
- Any write actions from the UI (creating/deleting playlists, retrying tasks) — this bootstrap is read-only browsing.
- Push-based updates (SSE/WebSockets) — polling is sufficient for this scope.
- Client-side routing/deep-linkable URLs for the three views — the app is one page with in-memory view state.

## Decisions

### Frontend: React + Vite, embedded via `rust-embed`, no router library
A `web/` directory holds a standalone Vite + React project (its own `package.json`, not a Cargo workspace member). `npm run build` produces `web/dist/`. A `rust-embed` `#[derive(RustEmbed)] #[folder = "web/dist/"]` type embeds that output into the `yarrtube` binary at compile time; an axum handler serves `/` from the embedded `index.html` and any other path under a small asset-serving branch from the embedded files by matching the request path against embedded file names, falling through to a 404 for anything unmatched.

No router library (e.g. react-router): the app has exactly three views (playlist list, playlist detail, tasks tab) managed via component state, not distinct URLs — simplest fit for this scope, and avoids a dependency that would matter more with actual deep-linkable pages.

Alternatives considered:
- **Serve from a static directory (`tower-http::ServeDir`)** instead of embedding: rejected because it reintroduces a runtime file-path dependency the Dockerfile doesn't have today, and complicates local `cargo run -- serve` from an arbitrary working directory.
- **No framework (vanilla JS)**: reasonable and simpler to build, but rejected per explicit product direction — the user expects this UI to grow (more views, eventually in-browser playback), and a component model pays for itself sooner than it would need to be introduced later via a rewrite.

### Build ordering: frontend builds before the Rust build, everywhere
Because `rust-embed` needs `web/dist/` to exist at compile time, `web/dist/` must be built before *any* `cargo build`/`cargo test`/`cargo clippy` invocation, not just before packaging the final image:

- **Dockerfile**: gains a `node:...` build stage that runs `npm ci && npm run build` in `web/`, producing `web/dist/`; the existing Rust build stage copies that output in before `cargo build --release`. The runtime stage is unchanged (still just the compiled binary, no Node).
- **CI** (`.github/workflows/ci.yml`): gains an `npm ci && npm run build` step (or a Node setup + build job) before the existing fmt/clippy/build/test steps, since all of them compile the crate.
- **Local dev**: `DEVELOPMENT.md` gains a one-time `cd web && npm ci && npm run build` step ahead of `cargo build --release`, documented alongside the existing `yt-dlp`/`.env` setup steps.
- `web/dist/` is git-ignored (build output), matching `target/`.

Alternative considered: checking `web/dist/` into git so `cargo build` never depends on Node being installed. Rejected — a stale or forgotten rebuild would silently serve outdated UI code with no compiler error to catch it, worse than requiring Node for a full local build.

### Route layout: JSON API under `/api/`, UI at `/`
`src/http/mod.rs` nests the existing `playlists_router` (renamed conceptually to "the JSON API router", covering `/playlists`, `/custom-playlists`, and the new `/playlists/{id}/videos`, `/tasks`) under `/api` in `serve.rs`. `GET /status` stays bare (infrastructure health check, not part of the business API). The SPA's root/asset handler is mounted at the router's remaining, unprefixed paths.

### New query surfaces follow existing conventions, not new abstractions
- `VideoService` (`src/domain/video/service.rs`) gains `list_videos(&self, playlist_id: &PlaylistId) -> Result<Vec<Video>, ListVideosError>` — verifies the playlist exists (reusing the existing `playlist_repository` it already holds) before delegating to `video_repository.list_for_playlist`, mirroring how `reconcile_playlist` already checks existence first.
- A new `domain/task/service.rs` (`TaskService`) is added, holding just the `TaskRepository` port, with one method: `list_non_completed(&self) -> anyhow::Result<Vec<ScheduledTask>>`. This keeps the "HTTP calls exactly one domain method" convention even though today it's a thin passthrough — the natural place for this to grow (e.g. exposing dead-lettered tasks later) is here, not in the HTTP handler.
- `TaskRepository` gains `list_non_completed(&self) -> anyhow::Result<Vec<ScheduledTask>>`, implemented as `SELECT ... WHERE status IN ('pending', 'running')` — deliberately not filtered by `run_at`, unlike `list_eligible`.
- `AppState` gains `task_service: TaskService`.
- New `http/videos/` and `http/tasks/` modules follow the existing `http/<resource>/{mod.rs,dto.rs}` shape; not-found and validation errors map to `400 Bad Request` via the existing `error_response` helper, consistent with how `playlists`/`custom_playlists` already handle "not found" (this codebase does not use `404` for missing domain entities).

### Polling interval: 3 seconds, fixed
The playlist-detail (videos) and tasks views poll their endpoint every 3 seconds via `setInterval` + `fetch` while visible. No backoff/visibility-based pausing for this bootstrap — traffic is a single browser tab against a local/NAS daemon, not a concern at this scale.

## Risks / Trade-offs

- **[Breaking change]** Any existing `curl`/automation use of the documented endpoints (`POST /playlists`, etc.) breaks the moment this ships. → Mitigated by updating every README example to `/api/...` in this same change; there's no way to make a path move non-breaking, so it ships as a single coordinated update rather than a deprecation period (matches this project's pre-1.0, no-external-consumers stage).
- **[Build fragility]** A fresh clone now needs Node installed before `cargo build` succeeds at all (even `cargo test`), which is a new local-setup requirement. → Documented explicitly in `DEVELOPMENT.md`; accepted as the cost of a real UI over a build-free one, per the frontend decision above.
- **[Polling load]** Fixed 3s polling from every open tab adds constant low-level query load to SQLite. → Acceptable at this project's scale (a single-user NAS daemon); revisit if it ever matters.

## Migration Plan

No data migration — this change is additive (new tables/columns: none; new endpoints and a route prefix move only). Deploying a new image immediately serves the new UI and the relocated API; there is no gradual rollout mechanism (single-container deploys per the README), so this ships in one release like any other change to this project.

## Open Questions

None — every ambiguity above was resolved in Decisions rather than deferred.
