# Tasks

## 1. Shared panic message helper

- [x] 1.1 Move `panic_message` from `task_executor.rs` to `src/infrastructure/shared/panic_message.rs` and use it from the task executor; verify `cargo test task_executor` passes

## 2. Consumer catches subscriber panics

- [x] 2.1 Wrap each `subscriber.handle` call in `DomainEventsConsumer::poll_once` with `catch_unwind`, mapping a panic to `subscriber panicked: <msg>` so it flows into the retry/dead-letter path
- [x] 2.2 Add consumer tests: a panicking subscriber increments retries with the panic message as `last_error`; panicking on the 5th attempt dead-letters; a second pending event is still processed in the same poll. Verify `cargo test domain_events_consumer` passes

## 3. Plex client per request

- [x] 3.1 Replace the shared `client` field in `HttpPlexLibraryRepository` with a `client()` helper that builds a blocking client with `REQUEST_TIMEOUT` per request, returning an error instead of panicking on build failure; verify `cargo test plex_library_repository` passes

## 4. Verification

- [x] 4.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `scripts/run-tests.sh` (all suites) and verify they pass
