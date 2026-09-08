## Why

Every daemon-facing message today is a plain `println!`/`eprintln!` (48 call
sites across `serve.rs`, both domain services, and the event/task
infrastructure). There is no way to distinguish routine activity from
warnings or errors, no way to turn verbosity up or down without a rebuild,
and no way to keep log noise out of test output. A leveled, structured
logger fixes all three and is a prerequisite for reasoning about the daemon
in production (e.g. on a NAS via `docker logs`).

## What Changes

- Introduce `tracing` + `tracing-subscriber` (the Rust ecosystem standard for
  leveled, structured logging) as the project's logger.
- Initialize a global subscriber once, at daemon startup (`serve::run`),
  reading its level filter from the `RUST_LOG` environment variable (the
  standard `tracing`/`env_logger` convention), defaulting to `info` when
  unset.
- Replace every `println!`/`eprintln!` call site in the daemon's operational
  path — `serve.rs`, `domain/playlist/service.rs`, `domain/video/service.rs`,
  and `infrastructure/repositories/{task_executor,domain_events_consumer,
  sqlite_task_repository,sqlite_event_repository}.rs` — with the matching
  `tracing` macro (`info!`/`warn!`/`error!`/`debug!`), attaching the
  interpolated values as structured fields instead of formatting them into
  the message string.
- Tests get no subscriber installed, so log output is a no-op there ( a
  `tracing` macro with no global subscriber registered short-circuits to a
  cheap no-op) — no test code changes needed and no dependency on a
  logging/mocking crate for this.
- **Out of scope**: the one-shot `download` and `update-ytdlp` CLI commands
  (`cli/download_command.rs`, `cli/ytdlp_update.rs::run`, and
  `infrastructure/client/youtube_downloader_client.rs`) keep their
  `println!`/`eprintln!` calls unchanged — that output is the command's
  actual user-facing result (progress, summary, errors), not diagnostic
  logging, and the `playlist-download` spec already requires it be printed
  to the console unconditionally.

## Capabilities

### New Capabilities
- `logging`: introduces leveled, structured logging for the daemon —
  configurable verbosity via an environment variable, a default level, and
  no log output in the test environment.

### Modified Capabilities
(none — the `daemon` spec's existing requirements already describe behavior
abstractly as "the daemon logs X"; this change satisfies them with a real
logger instead of `println!`, without changing what must be observably
logged.)

## Impact

- **Dependencies**: adds `tracing` and `tracing-subscriber` (with the
  `env-filter` feature) to `Cargo.toml`.
- **Code**: `serve.rs` (subscriber init call added to `run()`); the two
  domain services; four infrastructure repository/executor files. ~48
  `println!`/`eprintln!` call sites replaced.
- **Runtime behavior**: log verbosity becomes configurable via `RUST_LOG`
  without a rebuild; default verbosity (`info`) is chosen to match today's
  visible output. No change to HTTP API, CLI argument surface, or persisted
  data.
- **Docs**: `README.md` gains a mention of `RUST_LOG` alongside the other
  environment variables.
