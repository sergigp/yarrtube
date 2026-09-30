# Plex Collections Sync

## Why

Browsing a downloaded channel or playlist in Plex is a poor experience: the
library is a flat grid of thousands of loose videos. The NFO files yarrtube
already writes cannot fix this — Plex's NFO agent ignores the collection
(`<set>`) tag, so collections must be created through the Plex server HTTP
API. A manual API experiment against a real Plex server confirmed the
missing piece: one collection per tracked playlist/channel (plus the
library's "hide items which are in collections" option) turns the library
into one tile per playlist/channel, with correct in-playlist ordering and
autoplay across episodes.

## What Changes

- New optional Plex integration, enabled only when `YARRTUBE_PLEX_URL` and
  `YARRTUBE_PLEX_TOKEN` are set (plus the target library section). When
  disabled, yarrtube behaves exactly as today.
- New global recurring task (like `UpdateYtdlp`, unlike the per-entity
  reconcile tasks) that converges Plex collections toward yarrtube's state
  in a single pass:
  - fetches the Plex section's items once and maps them to yarrtube videos
    by YouTube ID (Plex's NFO agent exposes yarrtube's
    `<uniqueid type="youtube">` as a `Guid youtube://<id>` on every item);
  - for each tracked playlist and channel, upserts a collection: creates it
    if missing (with alphabetical sort, so yarrtube's position-prefixed
    `sorttitle` yields playlist order), adds downloaded videos Plex has
    scanned, removes members no longer in yarrtube's state.
  - The pass is idempotent and eventually consistent: videos not yet
    downloaded or not yet scanned by Plex are simply picked up on a later
    pass. Per-collection failures are logged and skipped, never abort the
    pass.
- New subscribers on `playlist_deleted` / `channel_deleted` that delete the
  corresponding Plex collection (symmetric with existing file cleanup).
- The sync is fully decoupled from reconciliation: reconcile passes are not
  slowed down and never talk to Plex.

## Capabilities

### New Capabilities

- `plex-collections`: keeping one Plex collection per tracked playlist and
  channel in sync with yarrtube's downloaded videos, including creation,
  membership convergence, ordering, deletion, and the enable/disable
  configuration switch.

### Modified Capabilities

<!-- none: reconciliation, downloads, events and existing tasks keep their
     current requirements; this change only adds a new consumer of their
     state. -->

## Impact

- New Plex HTTP client/adapter in `infrastructure/` (server identity,
  section items listing, collection create/add/remove/delete, collection
  sort pref) authenticated via `X-Plex-Token`.
- New domain service for the convergence pass and a new `TaskKind` variant
  wired into the task executor and scheduled at startup by `serve`.
- Two new event subscribers registered in `application/subscribers`.
- New env vars read in `serve.rs`: `YARRTUBE_PLEX_URL`,
  `YARRTUBE_PLEX_TOKEN`, section selection, and a sync interval.
- No schema/API changes to existing endpoints; no behavior change when the
  integration is disabled.
- README gains setup docs (obtaining a token, recommending the "hide items
  which are in collections" library setting).
