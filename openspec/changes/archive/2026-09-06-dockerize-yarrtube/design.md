## Context

See proposal.md - Why. Today `yarrtube` is a single synchronous Rust binary (`clap` for args, blocking `reqwest` for the YouTube Data API, `std::process::Command` shelling out to `yt-dlp`) with no persistence and no server. This design covers how it becomes a long-running container.

## Goals / Non-Goals

**Goals:**
- One container shape that already fits the eventual daemon (scheduler + REST API), so it isn't rebuilt from scratch next iteration.
- Keep today's `download` behavior byte-for-byte the same; only its invocation context (inside a running container) changes.
- Introduce the async/web stack (`tokio` + `axum`) once, scoped narrowly, rather than deferring it and having to retrofit it later.

**Non-Goals:**
- No task scheduler/queue implementation - `serve`'s hearteat loop is a placeholder log line, not a real scheduler.
- No REST API beyond `/status` - no playlist/subscription CRUD yet.
- No SQLite schema, no dedup/tracking logic, no persisted DB volume - the database is only proven to be openable and queryable.
- No channel support - `download` still only accepts playlist URLs.

## Decisions

### Single binary, three subcommands
`yarrtube` gains `serve` and `update-ytdlp` alongside the existing bare `<playlist_url> <output_path>` invocation (kept as-is or moved under a `download` subcommand - an implementation detail for tasks.md). All three are plain `clap` subcommands in the same crate, not separate binaries or crates.
- **Why**: The user's stated direction is workers/tasks/queues in the future, but "bootstrapped as normal code" now - i.e., don't build a task-runner abstraction yet, just add the second task the same way the first one was written. Splitting into multiple crates now would be structure built for a scheduler that doesn't exist yet.
- **Alternative considered**: separate `yarrtube-daemon` binary. Rejected - doubles build/deploy surface for no present benefit, and the daemon and the tasks will likely need to share code (e.g. the SQLite handle) once the scheduler is real.

### Async scoped to `serve` only
`serve` builds a `tokio` runtime and runs `axum` inside it. `download` and `update-ytdlp` remain fully synchronous/blocking, called from a non-async `main`.
- **Why**: `axum`/`tokio` is the standard, well-documented Rust web stack and the natural fit for the future REST API, so adopting it now (rather than a minimal sync HTTP crate) avoids a framework swap later. Scoping it to only the `serve` branch avoids forcing the rest of the codebase (and someone less familiar with Rust) to reason about async everywhere.
- **Alternative considered**: a minimal blocking HTTP server (e.g. `tiny_http`) for just `/status`. Rejected - would very likely be thrown away the moment real API endpoints are added.

### yt-dlp delivered as a standalone binary, not via pip
`update-ytdlp` downloads yt-dlp's self-contained Linux release binary from GitHub releases and swaps it in place; the image does not install Python/pip, and does not bake a copy of yt-dlp in at build time.
- **Why**: Avoids a second language runtime and package manager in the image; the fetch-and-replace logic is a plain HTTP GET + file write, reusing the `reqwest` dependency already in the project. Not baking yt-dlp into the image keeps the Dockerfile simpler and relies on `serve`'s startup self-update (see daemon spec) to populate it on first boot.
- **Trade-off**: first container start requires outbound network access before `download` can succeed. Acceptable for a NAS with normal internet access; a future iteration could bake in a fallback copy if this becomes a problem.

### Base image: Debian-slim multi-stage build over Alpine
Build stage: `rust:slim-bookworm` (or similar Debian-based Rust image). Runtime stage: `debian:bookworm-slim` plus `ffmpeg` and `ca-certificates` installed via `apt`.
- **Why**: `reqwest`'s default TLS backend depends on OpenSSL, which is friction-prone to cross-compile/link against musl (Alpine's libc). Debian-slim avoids that entirely, keeping the Dockerfile boring and easy to follow for someone not deeply familiar with Rust's build tooling. `ffmpeg` is needed for yt-dlp to mux downloaded formats and is a simple `apt install` on Debian.
- **Alternative considered**: Alpine, for a smaller image. Rejected for now given the TLS/musl friction; image size isn't a binding constraint on a NAS.

### SQLite is unpersisted this iteration
The daemon opens a SQLite file inside the container's own filesystem (not a mounted volume) purely to run a connectivity-check query at startup.
- **Why**: Nothing durable is stored in it yet, so persistence isn't needed; adding a volume for a file with no real data would be premature. The video output directory is the only volume this iteration introduces.
- **Note**: once real tracking data lands (a future iteration), the DB file's location will need to move onto a mounted volume - tasks.md / a future change should not assume the current path is final.

## Risks / Trade-offs

- [Startup self-update failing silently masks a stale yt-dlp for a long time] -> Mitigated by making the daemon log update failures clearly (per the daemon spec's failure scenario), so failures are visible in container logs even though they don't stop the container.
- [No persisted SQLite volume means the "database works" check doesn't prove anything about durability] -> Acceptable for this iteration since no durable data exists yet; flagged above so it isn't forgotten when real tracking data is added.
- [Adopting tokio/axum now adds a dependency and a bit of complexity before it's strictly needed for `/status` alone] -> Accepted deliberately to avoid a second migration later; scope is kept narrow (only `serve` is async).

## Migration Plan

This is a net-new deployment path (no prior container existed), so there's nothing to migrate from. Rollback is simply not running the container and continuing to use the existing local binary as documented in the current README.
