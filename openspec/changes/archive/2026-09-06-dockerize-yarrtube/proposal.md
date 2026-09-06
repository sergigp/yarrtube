## Why

Yarrtube today is a one-shot local binary you run by hand from a dev machine. The goal is a container that lives permanently on a NAS: it needs to stay running (so ad hoc work can be sent into it), prove that its own dependencies (SQLite, yt-dlp) are actually working, and expose the first sliver of an HTTP surface that the future scheduler and REST API (playlist/subscription management) will grow out of. This change packages the existing playlist downloader into that long-running container shape without yet building the scheduler or REST API themselves.

## What Changes

- Package `yarrtube` as a Docker image: multi-stage build (`rust:slim-bookworm` build stage, `debian:bookworm-slim` runtime stage with `ffmpeg` and `ca-certificates`), producing a container meant to run continuously on a NAS.
- Add a `serve` subcommand that becomes the container's default entrypoint (a placeholder for the future task scheduler/worker). On start it: runs the yt-dlp self-update task once, opens/creates a local SQLite file and runs a trivial query to confirm it works, starts an HTTP server, and logs a periodic heartbeat.
- Add a `GET /status` endpoint (via `axum`/`tokio`, scoped to the `serve` subcommand) that returns `200 OK`, exposed outside the container on a published port. This is the first endpoint of what will become the REST API.
- Add an `update-ytdlp` subcommand: a self-contained task that downloads yt-dlp's standalone Linux release binary from GitHub and replaces the local copy in place. No Python/pip dependency. Runnable both manually (`docker exec`) and automatically once at `serve` startup.
- Keep the existing `download <playlist_url> <output_path>` behavior unchanged; it continues to be invoked ad hoc via `docker exec` against the running container, writing into a volume mounted for Plex, and still does not touch SQLite.
- Add a volume mount for the video output directory (Plex-visible); the SQLite file is not volume-mounted in this iteration (not required to persist yet, since nothing durable is stored in it).

## Capabilities

### New Capabilities
- `container-image`: Docker packaging of yarrtube — multi-stage build, runtime base image and dependencies, default entrypoint, and volume/port surface for running the container on a NAS.
- `daemon`: The `serve` subcommand's long-running behavior — startup sequence (self-update yt-dlp, verify SQLite connectivity), periodic heartbeat logging, and hosting the HTTP server. This is the placeholder the future task scheduler will be built into.
- `status-endpoint`: The `GET /status` HTTP endpoint served by the daemon, returning `200 OK` to confirm the service is reachable from outside the container.
- `ytdlp-self-update`: The `update-ytdlp` task — fetches yt-dlp's latest standalone Linux binary from its GitHub releases and replaces the binary the `download` task shells out to.

### Modified Capabilities
- None. `playlist-download`'s requirements (CLI invocation, API key config, playlist resolution, sequential download, per-video failure isolation) are unchanged; only where/how the binary is invoked changes (inside a running container via `docker exec` instead of directly on a dev machine), which is a deployment detail, not a behavior change.

## Impact

- `Cargo.toml`: new dependencies for the HTTP server/async runtime (`axum`, `tokio`) and SQLite access (e.g. `rusqlite`).
- `src/`: new `serve` and `update-ytdlp` subcommands alongside the existing `download` invocation, dispatched from a single binary; existing `cli.rs`/`downloader.rs`/`youtube_api.rs` behavior untouched.
- New `Dockerfile` (multi-stage) and `.dockerignore` at the repo root.
- Deployment surface: a published HTTP port for `/status` and a mounted host directory for downloaded videos; no persisted volume for the SQLite file in this iteration.
- Out of scope for this change: the real task scheduler/queue, the rest of the REST API (playlist/subscription CRUD), channel support, and download-level SQLite tracking/dedup — all deferred to future iterations.
