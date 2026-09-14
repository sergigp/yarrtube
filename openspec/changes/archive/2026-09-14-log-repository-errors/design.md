## Context

See proposal.md - Why/What Changes for motivation and file list. Today no
file in `src/infrastructure/repositories/` logs on a failure path; every
fallible method returns via `?`/`.context("...")` straight to its caller.
The only precedent for error-path logging in the codebase lives in
`src/infrastructure/repositories/task_executor.rs` and
`domain_events_consumer.rs`, which log generically
(`error!(error = %e, "poll failed")`) around repo calls but don't know
which specific query/file/request failed.

## Goals / Non-Goals

**Goals:**
- Every failure branch in every repository implementation logs at the
  point of failure, once, with enough structured context (identifiers +
  the underlying error) to diagnose it from production logs alone.
- Reuse the existing `.context("...")` wording as the log message so there
  is one source of truth for "what failed", not two drifting strings.
- Never let the YouTube API key leak into a log line.

**Non-Goals:**
- No change to control flow, retry behavior, or what any method returns.
- No fix for the suspected SQLite busy/locking root cause, and no HTTP-layer
  logging — both explicitly deferred per proposal.md.
- No new log aggregation/format — plain `tracing`, existing subscriber.

## Decisions

**Log level: always `error!`, never `warn!`, at the repository layer.**
The `logging` capability's "Failure is emitted at error level" requirement
is about the operation in front of us failing to complete, not about
whether an upstream caller will retry it. `task_executor.rs`/
`domain_events_consumer.rs` already log `warn!` vs `error!` at their own
layer based on retry-vs-dead-letter outcome — that's a separate, coarser
signal about task/event lifecycle. A repo call logging `error!` and the
orchestrator separately logging `warn!("task failed, retrying")` for the
same underlying failure are not duplicates; they answer different
questions ("what broke" vs "what happens to the task next"). This avoids
inventing a per-method judgment call about which failures are "transient."

**Log once, at the outermost public trait-method boundary — not inside
private row-mapping helpers.** `sqlite_playlist_repository.rs`'s
`row_to_playlist` and `sqlite_video_repository.rs`'s `row_to_video` already
attach their own `.context("failed to parse stored created_at"/...)`, and
their callers (`list`, etc.) wrap that again with
`.context("failed to read playlist row"/...)`. Logging inside the helper
*and* at the call site would print two log lines for one root cause. Only
the public trait method logs; it has the fully-formed `anyhow::Error`
(all context layers included) and the request-level identifiers
(`playlist_id`, etc.) that the helper doesn't have.

**Mechanism: `.inspect_err(|e| tracing::error!(...))` chained right after
each `.context("...")` (or in place of constructing `anyhow::anyhow!(...)`
in the filesystem repo), not a wrapping `match`/`if let Err` block.** Keeps
the log line textually next to the failure it describes, keeps the
existing `?`-propagation style, and avoids restructuring method bodies.
Example shape (`sqlite_video_repository.rs::delete_all_for_playlist`):
```rust
conn.execute("DELETE FROM videos WHERE playlist_id = ?1", params![playlist_id.as_str()])
    .inspect_err(|e| tracing::error!(playlist_id = %playlist_id, error = %e, "failed to delete videos for playlist"))
    .context("failed to delete videos for playlist")?;
```
Lock-poison branches (`anyhow::anyhow!("database lock poisoned")` in all
three SQLite repos) get the same treatment at the `.lock()` call site,
with whatever identifier is in scope (or none, for methods like `list()`
that don't have one yet).

**Redact the YouTube API key before logging any error from the three
YouTube repos.** The key is sent as a `key` query param via reqwest's
`.query(&[("key", self.api_key.as_str()), ...])`
(`youtube_playlist_repository.rs:43`, `youtube_video_repository.rs:63`,
`youtube_playlist_items_repository.rs:70`). `reqwest::Error`'s `Display`
includes the request URL for connection/timeout/status errors, so a bare
`error = %e` on the `.context("YouTube API request failed")` branch would
put the raw API key in production logs. Instead: log the identifier being
looked up (`playlist_id`/`video_id`) and, where the error is a known HTTP
status (the `anyhow::bail!("YouTube API request failed with status
{status}: {body}")` branches), the status code as its own field — not the
`reqwest::Error` itself. For the generic `.context("YouTube API request
failed")` `?`-propagation branch (network/timeout failures before a
response exists), log the identifier and a fixed message only, omitting
`error = %e` for that specific branch since it's the one path proven to
carry the URL+key. The `.context("failed to parse YouTube API response")`
branch (JSON decode failure) is safe to log with `error = %e` since that
error comes from `serde_json`/`reqwest::Error::json()` decode failures,
which don't embed the request URL.

**Constructors (`new()`/table-creation failures) are in scope.** They're
single `.context(...)` calls, no nested-helper double-logging risk, and
today a table-creation failure is only visible via the generic
`error!(error = %e, "failed to initialize application state")` in
`serve.rs`. Adding the specific log (e.g. `"failed to create videos
table"`) costs nothing and pinpoints which table/statement failed.

## Risks / Trade-offs

- [Log volume increase under sustained failure, e.g. a genuinely locked DB
  during an outage] → Acceptable: these are already-rare failure paths: no
  per-request/per-row logging in the success path, only on error.
- [Duplicate-looking log lines: a repo-level `error!` immediately followed
  by an orchestrator-level `warn!`/`error!` for the same failure in
  `task_executor.rs`/`domain_events_consumer.rs`] → Intentional per the
  log-level decision above; the two lines answer different questions and
  the repo-level one is strictly additive (it exists today with zero repo
  coverage).
- [Missing a leak path we didn't verify, e.g. `list_current_videos`'s
  pagination in `youtube_playlist_items_repository.rs` hitting the same
  `.context("YouTube API request failed")` branch] → Mitigated: the
  redaction rule above applies per branch pattern (bare `.context("YouTube
  API request failed")` on a `?`-propagated request error), not per
  method, so it covers `fetch_page` and any call site using that pattern
  uniformly.

## Migration Plan

No migration. This is additive logging behind the existing `tracing`
subscriber already initialized in `serve.rs` at default `INFO` level;
`warn!`/`error!` lines will be visible immediately on next deploy with no
config change. Rollback is a plain revert if log volume or content proves
problematic.
