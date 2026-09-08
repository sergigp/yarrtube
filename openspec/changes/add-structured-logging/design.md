## Context

See `proposal.md` - Why/What Changes for motivation and scope. Relevant
existing shape:

- 48 `println!`/`eprintln!` call sites, most already prefixed with a
  bracket tag (`[sync]`, `[tasks]`, `[events]`, `[startup]`, `[heartbeat]`)
  that groups messages by subsystem — the same grouping `tracing`'s
  `target` (auto-derived from the module path) gives for free.
- `serve.rs::run()` is the single entry point for the daemon (`main.rs`
  dispatches `Commands::Serve` to it); `build_application()` (also in
  `serve.rs`) is only ever called from `run()`, never from tests.
- The two `cli/` commands (`download`, `update-ytdlp`) and
  `youtube_downloader_client.rs` are out of scope per the proposal and are
  untouched by this design.
- No test currently asserts on stdout/stderr content from any of the files
  being touched (confirmed by inspection) — nothing needs to change on the
  test side to satisfy "no log output in tests" per rust-architect's
  outside-in testing convention ([[feedback_test_from_outside_in]]).

## Goals / Non-Goals

**Goals:**
- Pick one Rust logging stack and use it consistently across every touched
  call site.
- Make verbosity configurable without a rebuild, matching today's default
  visible output when unconfigured.
- Get "no log output in tests" for free, with no test-side code.

**Non-Goals:**
- Spans / distributed tracing across async task boundaries — this change
  only replaces flat print statements with flat log events. Spans are a
  natural future extension once there's a concrete need (e.g. correlating
  all log lines for one sync run) but aren't required by the current
  request.
- JSON or otherwise machine-parseable output — human-readable text was the
  explicit, confirmed choice for this change.
- Converting the two interactive CLI commands' console output (confirmed
  out of scope).
- Log rotation/shipping — out of scope; `docker logs` / stdout capture is
  the deployment's existing story and this change doesn't alter it.

## Decisions

### Library: `tracing` + `tracing-subscriber` (with `env-filter`)
The de facto standard for leveled, structured logging in Rust (vs. the
older `log` + `env_logger`, which has no first-class structured-field
support). `tracing`'s macros (`info!`, `warn!`, `error!`, `debug!`) accept
`field = value` pairs alongside the message, which directly satisfies the
"structured logger" requirement — e.g.
`tracing::info!(playlist_id = %id, "syncing playlist")` rather than
`println!("[sync] syncing playlist {id}")`.

### Verbosity control: `EnvFilter` reading `RUST_LOG`, default `info`
`tracing-subscriber::EnvFilter` is the standard mechanism and `RUST_LOG` is
its conventional variable name (also what `env_logger` uses) — operators
already familiar with the Rust ecosystem need no new documentation to guess
it. Built via
`EnvFilter::builder().with_default_directive(LevelFilter::INFO.into()).from_env_lossy()`
so a missing `RUST_LOG` falls back to `info` (matching today's default
visible output) and a malformed value degrades gracefully instead of
panicking at startup.

### Initialization: once, inside `serve::run()`, before any other startup step
The subscriber is process-global and must be installed exactly once. `run()`
is the only caller of everything else in `serve.rs` and is never invoked
from tests (confirmed above), so installing it as the first line of `run()`
cannot double-init or leak into a test binary. `cli::download_command::run`
and `cli::ytdlp_update::run` do not install a subscriber — consistent with
them staying on plain `println!`/`eprintln!`.

### Tests: no subscriber installed, relying on `tracing`'s built-in no-op default
When no global subscriber is set, `tracing`'s macros resolve against a
default no-op dispatcher and the log call is skipped (a cheap `is_enabled`
check, no formatting, no I/O). This gets the "noop logger in tests"
requirement for free — no `tracing-test`, no fake subscriber, no test
harness change. Explicitly rejected: pulling in `tracing-test` or hand-
rolling a null subscriber, since the default behavior already satisfies the
spec's "no log output in tests" requirement with zero added code.

### Call-site mapping: drop the bracket tags, keep the message text
Every `println!("[tag] message", ...)` becomes
`tracing::<level>!(<structured fields>, "message")`, dropping the `[tag]`
prefix — `tracing`'s default formatter already prints the emitting module's
path as the `target`, which is a strict superset of the hand-written tags
(`[sync]` → `yarrtube::domain::video::service`, etc.). Values that were
interpolated into the message string move to named fields instead (e.g.
`id` for a playlist id, `task_id`/`task_type` for tasks). Level is chosen
per the `logging` spec's Leveled Log Output requirement: successful/routine
paths → `info`, a skip/no-op path (e.g. "playlist no longer exists,
skipping sync") → `info` or `debug` by judgment at implementation time,
retry/permanent-failure and startup-check failures → `warn`/`error`
respectively (permanent failure and startup/poll failure → `error`; a
retryable failure that will be retried → `warn`).

## Risks / Trade-offs

- **Dropping the bracket tags changes the exact text of log lines** →
  Mitigation: nothing in the codebase or specs parses these lines (grep
  confirmed no test asserts on stdout content); the `target` field replaces
  the same information in a more structured form.
- **`RUST_LOG` accepts per-module directives (e.g. `yarrtube::domain=debug`)
  which is more power than most operators will use** → Mitigation: this is
  the standard `tracing` behavior, documenting just the common case
  (`RUST_LOG=debug`) in the README is enough; no extra code needed to
  support or restrict it.
- **Choosing log level per call site is a judgment call with no automated
  check** → Mitigation: the `logging` spec's scenarios (routine → info,
  failure → error) give a concrete rule to apply at each site during
  `tasks.md` execution.

## Open Questions

None — the two decisions that would have changed scope or approach (CLI
output in/out of scope, log output format) were resolved with the user
before writing this design.
