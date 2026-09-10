# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Yarrtube downloads every video in a YouTube playlist to a local directory via
`yt-dlp`. It's packaged as a Docker image meant to run continuously (e.g. on a
NAS), exposing an HTTP API to track playlists plus a `serve` daemon that syncs
them on an interval. See `README.md` for the full deployment/runtime story
(Docker, docker-compose, environment variables, releasing).

## Commands

```bash
cargo build --release                 # build
cargo test --locked                   # run all tests
cargo test <substring>                # run a single test / module by name filter
cargo fmt --all -- --check            # formatting check (CI enforces this)
cargo clippy --all-targets --all-features --locked -- -D warnings   # lint (CI enforces this, warnings fail)
```

CI (`.github/workflows/ci.yml`) runs fmt, clippy, build, and test on every PR
and push to `main`; match that locally before pushing. Releases are cut by
pushing a `vX.Y.Z` tag (see `.github/workflows/release.yml` and `README.md`),
never by hand.

Run the binary directly for local (non-Docker) development:

```bash
./target/release/yarrtube serve
./target/release/yarrtube download <playlist_id> <output_path>
./target/release/yarrtube update-ytdlp
```

Requires `yt-dlp` on `PATH` and a `.env` (copy `.env.example`) with
`YOUTUBE_API_KEY` set.

## Architecture

This project follows the conventions in the `rust-architect` skill
(`.claude/skills/rust-architect/SKILL.md`) — read that for the full rationale
(layering, file naming, port placement, CQS repository contract, testing
style). Apply it by default; the notes below are yarrtube-specific instances
of it, not a replacement.

**Layers**: `domain/<aggregate>/` (playlist, video, task, shared) holds
value objects, entities, `errors.rs`, and one `service.rs` per aggregate with
all orchestration. `http/` and `cli/` are adapters — parse input, call one
domain method, map the result to a response/exit code, nothing more.
`infrastructure/repositories/` implements the ports domain services inject
(SQLite repositories, the YouTube API repositories, `SystemClock`);
`infrastructure/client/` and `infrastructure/shared/` hold everything else
(see the skill for the repositories/client/shared distinction).

**Domain events and background execution** (`src/domain/event.rs`,
`src/subscribers/`, `src/tasks/`, `src/infrastructure/repositories/
domain_events_consumer.rs` and `task_executor.rs`): domain services emit
`DomainEvent`s and persisted `Task`s through injected ports rather than
calling other aggregates' services directly. Two background loops, started in
`serve.rs` alongside the HTTP server, poll SQLite and dispatch:

- `DomainEventsConsumer` matches an event's `event_type()` against a
  `SubscriberRegistry` (built in `src/subscribers/mod.rs`) and runs each
  matching subscriber — e.g. `PlaylistCreated` → `SyncPlaylistOnPlaylistCreated`,
  which schedules a sync task rather than syncing inline.
- `TaskExecutor` matches a task's type against a `HandlerRegistry` (built in
  `src/tasks/mod.rs`) and runs the one handler for it — e.g. `sync_playlist` →
  `SyncPlaylistTask`, which drives `VideoService` to pull playlist items from
  YouTube and kick off downloads.

Adding a new reaction to an event or a new task type means adding a
subscriber/handler file and registering it in the corresponding `registry()`
function — the consumer/executor plumbing itself doesn't change.

**Composition root**: `serve.rs::build_application()` wires every
repository, service, subscriber registry, and task registry by hand (no DI
framework) and returns an `Application` bundling the HTTP `AppState`, the
event consumer, and the task executor, which `run_async` spawns as concurrent
tokio tasks alongside the HTTP server and a heartbeat. `main.rs` just parses
CLI args and dispatches to `cli::download_command`, `serve::run`, or
`cli::ytdlp_update`.

## Spec-driven development (OpenSpec)

This repo uses OpenSpec (`openspec/`) for spec-driven change management —
proposals and specs live under `openspec/changes/` and `openspec/specs/`
(one directory per capability: `playlist-crud`, `playlist-sync`,
`playlist-download`, `task-scheduling`, `domain-events`, `daemon`,
`status-endpoint`, `container-image`, `ytdlp-self-update`). Use the
`openspec-*` / `opsx:*` skills (propose, apply, update, sync-specs, archive,
explore) when starting, continuing, or finalizing a spec'd change rather than
editing `openspec/` files by hand.
