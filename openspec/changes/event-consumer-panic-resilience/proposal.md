# Proposal

## Why

On 2026-11-01 the Plex HTTP client's internal thread died. Every later Plex call then panicked, and because the domain events consumer does not catch subscriber panics, event 5320 (`video_subtitles_fetched`) was re-dispatched every 5 s (4,240 times) without its retry counter ever moving. It never dead-lettered, and every event queued after it in the batch starved, until a restart. A single dead client should cost a few retries, not the whole event queue.

## What Changes

- The domain events consumer treats a subscriber that panics as a failed attempt, exactly like one that returns an error: the retry counter increments, the event dead-letters after 5 attempts, and the rest of the batch is still processed.
- The Plex library repository no longer shares one long-lived HTTP client: a failure of the client's machinery during one request no longer breaks every later Plex request until restart.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `domain-events`: a subscriber panic counts as a failed attempt (retry, then dead-letter) and does not stop other pending events from being processed.
- `plex-library-scan`: a broken Plex request does not poison later ones; Plex requests recover without a restart.

## Impact

- `src/infrastructure/repositories/domain_events_consumer.rs` (panic capture per subscriber).
- `src/infrastructure/repositories/task_executor.rs` (panic-message helper shared with the consumer).
- `src/infrastructure/repositories/plex_library_repository.rs` (HTTP client per request).
- No API, schema or configuration changes.
