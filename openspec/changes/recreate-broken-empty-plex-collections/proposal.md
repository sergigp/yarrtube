## Why

Two channel collections in production (PBS Space Time `10607`, Control de
Misión `10591`, section 21) are stuck in a state where Plex answers every
add (`PUT /library/collections/{key}/items`) and remove
(`DELETE .../items/{key}`) with `400 Bad Request`. Every sync pass fails
them again, so those channels never get their Plex tile filled. Both have
been empty since they were created on 2026-09-30 and never got
`collectionSort` set, i.e. their original create never fully completed. The
videos themselves are fine: adding them to a fresh collection works, and
fresh empty collections (or ones created against a missing rating key)
accept adds, so the root cause inside Plex is unknown and the broken state
cannot be prevented, only recovered from. Today the error also only says
`failed with status 400 Bad Request`, dropping whatever Plex said.

## What Changes

- When a sync pass finds an existing collection with **no members** while
  the playlist/channel has scanned videos to put in it, it deletes that
  collection and creates it fresh with the desired members (which also
  configures alphabetical sorting), instead of adding to it. An empty
  collection holds nothing worth preserving, so recreating it is safe and
  self-heals the broken state on the next pass.
- Failed Plex requests include Plex's response body in the error message,
  so the next unexplained Plex failure is diagnosable from logs.

Out of scope: the one-off timeout listing section 19 (1145 items list in
~2.3s; a busy Plex exceeded the 10s timeout once and the next pass retries).

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `plex-collections`: "Collection membership converges to yarrtube's state"
  gains the rule that an existing empty collection with desired members is
  recreated rather than added to.

## Impact

- `src/domain/services/plex_collection_reconciler.rs` — `converge_members` /
  `reconcile_collection` branch for empty existing collections.
- `src/infrastructure/repositories/plex_collection_repository.rs` —
  `ensure_success` reads the response body into the error; the fake
  repository may need a way to simulate a collection rejecting adds.
- No API, config, DB or migration changes. Existing broken collections in
  production heal on the first pass after deploy.
