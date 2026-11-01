# Design

## Context

- `DomainEventsConsumer::poll_once` calls each subscriber directly. A panic unwinds out of `poll_once`; `run` only logs it (`poll task panicked`), so `event.fail()` is never called and the event stays eligible with an unchanged counter. `TaskExecutor::dispatch` already wraps handlers in `catch_unwind` and maps a panic to `handler panicked: <msg>`.
- `HttpPlexLibraryRepository` builds one `reqwest::blocking::Client` in `new`. A blocking client runs its requests on a private runtime thread; once that thread dies, every `send` panics with `core thread panicked` for the life of the client. The YouTube repositories already build a client per call.

## Goals / Non-Goals

**Goals:**
- A subscriber panic behaves like a subscriber error (see the `domain-events` delta).
- A dead Plex client cannot outlive the request that broke it.

**Non-Goals:**
- Finding why the Plex client thread died on 2026-11-01 (needs the original panic line from the production logs).
- Migrating to the async reqwest client.

## Decisions

- **Catch per subscriber, not per poll.** Wrap each `subscriber.handle` in `catch_unwind(AssertUnwindSafe(..))` and turn the payload into `anyhow!("subscriber panicked: {msg}")`. The other subscribers for that event still run and the failure flows into the existing `fail()` → retry/dead-letter path. Catching around `poll_once` would lose which event failed. `AssertUnwindSafe` is fine: subscribers hold `Arc`s to repositories that commit or roll back per call, same reasoning as the task executor.
- **Share `panic_message`.** Move it from `task_executor.rs` to `infrastructure/shared/panic_message.rs` so both the executor and the consumer use it.
- **Plex client per request** (over keeping a shared client and rebuilding it after a caught panic). It matches the YouTube repositories and needs no locking or panic handling in the repository. A client is built in a private `client()` helper with `REQUEST_TIMEOUT`; a build failure becomes an error for that request instead of an `expect` at startup.

## Risks / Trade-offs

- [No connection reuse; each request spawns and joins a runtime thread] → Plex traffic is low-volume (scans, label sync of at most a few thousand edits per run), and thread spawn plus a TCP connect to a LAN server is milliseconds. Acceptable.
- [A panicking subscriber can still leave partial side effects] → Same as an erroring one today; subscribers are already required to be safe to re-run.

## Migration Plan

No data or config migration. Deploy normally; a restart clears any client already dead in a running container.
