## Why

Every tracked channel and playlist is reconciled on one global interval (`YARRTUBE_RECONCILE_INTERVAL_SECONDS`), but sources differ: a daily-uploading channel wants frequent checks, an archived playlist barely any. Making the interval a per-source setting, editable from the web UI, removes the env variable and lets each source be polled as often as it needs.

## What Changes

- Channels and playlists store their own reconcile interval in whole minutes, from 1 to 1440 (24 hours). Existing rows and new sources default to 180 minutes (3 hours).
- The recurring reconcile of a channel or playlist schedules its next run after that source's own interval, read when the pass finishes.
- Creating a channel or playlist accepts an optional `reconcile_interval_minutes`; channel and playlist responses include it.
- `PATCH /api/channels/{handle}` accepts `reconcile_interval_minutes` alongside quality and video limit.
- `PATCH /api/playlists/{id}` accepts `reconcile_interval_minutes`; **BREAKING** (API): `exclude_from_home` becomes optional, and a request carrying neither field is rejected as nothing to update.
- Changing a source's interval brings its pending reconcile forward to `now + new interval` when that is earlier than the run already scheduled; a later run is never pushed back.
- Web UI: the edit channel dialog gains a "Check for new videos every" field (number + minutes/hours unit); a new edit playlist dialog with only that field is reachable through "Edit settings" in playlist row menus and the playlist page header; both add dialogs show the field under "Advanced options", defaulting to 3 hours.
- **BREAKING** (config): `YARRTUBE_RECONCILE_INTERVAL_SECONDS` is removed with no fallback; a value set there is not carried over.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `channel-crud`: Create Channel accepts and stores a reconcile interval; Update Channel Settings can change it and brings the pending reconcile forward; List Channels reports it.
- `playlist-crud`: Create Playlist accepts and stores a reconcile interval; the exclude-from-home update becomes a general Update Playlist Settings requirement covering both settings and the reconcile bring-forward.
- `channel-video-sync`: Recurring Channel Reconciliation uses the channel's own interval instead of the shared configured one.
- `playlist-reconciliation`: Recurring Reconciliation uses the playlist's own interval; Configurable Reconcile Interval is removed.
- `web-ui`: Edit Channel Dialog gains the interval field; new Edit Playlist Dialog; Sidebar Row Actions and Detail View Page Header offer "Edit settings" on playlists; Add Dialog Download Options gains the interval field.

## Impact

- **Database**: migration `0009_reconcile_interval.sql` adds `reconcile_interval_minutes INTEGER NOT NULL DEFAULT 180` to `channels` and `playlists`.
- **Domain**: new `ReconcileInterval` value object; `Channel`, `Playlist`, `ChannelView` carry it; `ChannelCreator`, `PlaylistCreator`, `ChannelUpdater`, `PlaylistUpdater`, `ChannelVideoReconciler`, `PlaylistVideoReconciler` change; `ScheduledTask` gains a bring-forward transition; `TaskRepository` gains `find_pending`.
- **HTTP**: channel and playlist create/update request and response DTOs.
- **Composition root**: `serve.rs` drops `reconcile_interval_seconds()` and passes the task repository and clock to the updaters.
- **Web**: `api/types.ts`, `api/client.ts`, `api/queries.ts`, a new `lib/reconcileInterval.ts`, a shared `ReconcileIntervalField`, `EditChannelDialog`, new `EditPlaylistDialog`, `AddChannelDialog`, `AddPlaylistDialog`, `Sidebar`, `PlaylistDetail`, test fixture builders.
- **Docs/config**: `doc/INSTALLATION.md`, `doc/ARCHITECTURE.md`, `.env.example`.
- **Smoke tests**: the playlist edit flow.
