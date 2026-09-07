## Why

Yarrtube can only download a playlist ad hoc via the CLI; there is no way to
register a playlist for the daemon to track over time. The scheduler that
will eventually poll playlists and download new videos needs a persisted set
of playlists to query. This change adds that persistence via a small
hexagonal/DDD-flavored CRUD API on the daemon, and aligns the CLI's playlist
identifier with the same domain concept.

## What Changes

- Add `Playlist` domain entity (`id`, `name`, `created_at`) with validation
  in its constructor: `id` is a YouTube playlist ID, `name` must be
  non-empty and free of filesystem-unsafe characters (it will later double
  as the download folder name).
- Add `POST /playlists` - creates a playlist. Verifies the YouTube ID exists
  (via the YouTube Data API `playlists` resource) before persisting; rejects
  invalid names and duplicate IDs.
- Add `DELETE /playlists/{id}` - deletes a playlist by its YouTube ID.
- Add `GET /playlists` - lists all stored playlists.
- Introduce three ports (traits) implemented by infrastructure adapters:
  `PlaylistRepository` (SQLite-backed), `YoutubePlaylistLookup` (YouTube Data
  API-backed, existence check only), and `Clock` (system-clock-backed,
  supplies `created_at` so tests can assert on deterministic timestamps).
- Add integration tests that exercise the HTTP routes end-to-end against
  fake in-memory implementations of the ports (no real SQLite or network
  calls).
- **BREAKING**: refactor the `download` CLI command to take a raw YouTube
  playlist ID instead of a full playlist URL, reusing the same
  `YoutubePlaylistId` value object as the new API (the URL is derived from
  the ID, not parsed from it). Update the README's `download` usage
  accordingly.

## Capabilities

### New Capabilities
- `playlist-crud`: create, delete, and list playlists via the daemon's HTTP
  API, backed by a hexagonal domain/repository split.

### Modified Capabilities
- `playlist-download`: the CLI's `download` command now takes a YouTube
  playlist ID instead of a playlist URL.

## Impact

- New modules: a domain layer (`Playlist` entity, `YoutubePlaylistId` and
  `PlaylistName` value objects, `PlaylistRepository` / `YoutubePlaylistLookup`
  / `Clock` port traits), an HTTP controller layer for the new routes, and
  infrastructure adapters (SQLite repository, YouTube existence-check
  client, system clock).
- `src/serve.rs`: wires the new routes and concrete adapters into the axum
  `Router`/state; SQLite is now used for real storage, not just a startup
  connectivity check.
- `src/cli.rs`, `src/download_command.rs`, `src/youtube_api.rs`: `download`
  argument changes from playlist URL to playlist ID; `extract_playlist_id`
  is replaced by the shared `YoutubePlaylistId` value object.
- `README.md`: update the `download` usage examples.
- `Cargo.toml`: likely adds `tower` (or similar) as a dev-dependency for
  driving the axum router in integration tests.
