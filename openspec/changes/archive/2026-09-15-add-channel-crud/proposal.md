## Why

Yarrtube can only track curated YouTube playlists today. There's no way to point at a creator's channel and keep pulling their latest uploads automatically — every new video from a followed channel currently requires the user to notice it and add it to a playlist by hand. This change introduces a `Channel` aggregate so a channel can be tracked as a first-class entity, laying the CRUD/domain foundation a later sync-and-download iteration will build on, the same way `playlist-crud` preceded `playlist-sync`.

## What Changes

- Add a new `Channel` domain aggregate: `id` (the YouTube handle, e.g. `@somechannel`), `name` (channel display title), `youtube_channel_id` (the immutable YouTube channel ID), `quality` (reuses the existing shared `Quality` value object), `video_limit` (target number of most recent videos to keep synced), and `created_at`.
- Add `POST /channels`: accepts a channel handle or channel URL plus a name, quality, and video limit; resolves the handle against the YouTube Data API to confirm it exists and to capture the immutable channel ID and title; idempotent on the handle, mirroring `create_playlist`'s duplicate-ID behavior.
- Add `GET /channels`: lists every tracked channel.
- Add `DELETE /channels/{handle}`: removes a tracked channel. No update endpoint.
- Add a new `YoutubeChannelRepository` port and YouTube Data API implementation that resolves a handle to `{youtube_channel_id, title}`.
- Add a new SQLite `channels` table and repository.
- Add `ChannelCreated` and `ChannelDeleted` domain events, published on successful create/delete the same way `PlaylistCreated`/`PlaylistDeleted` are. No subscriber reacts to them yet — sync/download wiring is future scope.
- Add a "Channels" tab to the SPA with a list view and a create dialog, mirroring the existing Playlists tab.
- Out of scope for this change: syncing or downloading any channel videos, an update endpoint, and registering any subscriber/task handler for the new events.

## Capabilities

### New Capabilities
- `channel-crud`: HTTP endpoints to create, list, and delete tracked YouTube channels, backed by YouTube handle resolution and SQLite storage, including the `ChannelCreated`/`ChannelDeleted` domain events published on those operations.

### Modified Capabilities
None. `domain-events` already specifies the publish/dispatch mechanism generically and isn't tied to specific event types, so no existing spec's requirements change.

## Impact

- **New code**: `src/domain/channel/` (aggregate, value objects, `service.rs`, `errors.rs`), `src/http/channels/` (`mod.rs`, `dto.rs`), `src/infrastructure/repositories/sqlite_channel_repository.rs`, `src/infrastructure/repositories/youtube_channel_repository.rs`.
- **Modified code**: `src/domain/event.rs` (new `ChannelCreated`/`ChannelDeleted` variants), `src/http/mod.rs` (new routes wired into `api_router`), `serve.rs::build_application()` (wire the new repository/service into `AppState`), `web/src/` (new `Channel*` components, `api.js` functions, a new tab in `App.jsx`).
- **New external dependency**: calls to the YouTube Data API `channels` endpoint (`forHandle`), alongside the existing `playlists` and `playlistItems` calls.
- **Database**: new `channels` table; no changes to existing tables.
