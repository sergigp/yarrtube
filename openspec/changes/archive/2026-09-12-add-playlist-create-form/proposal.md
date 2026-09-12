## Why

Tracking a new playlist currently requires a manual `curl POST /api/playlists`
call — the SPA is read-only. Users copy a playlist ID or URL straight out of
their browser's address bar, which is usually the full YouTube URL rather
than the bare ID the API expects today. The SPA should let a user create a
playlist without leaving the browser, accepting whatever they paste.

## What Changes

- Add a "Create Playlist" form to the SPA that submits name, storage path,
  quality, and a single combined "playlist ID or URL" input to
  `POST /api/playlists`.
- Add `api.js` support for POSTing (the SPA's first mutation; today it only
  polls `GET` endpoints).
- **BREAKING**: `POST /api/playlists` accepts a YouTube playlist URL in
  addition to a bare playlist ID, via a new `PlaylistId::from_url_or_id`
  parser (mirroring the existing `VideoId::from_url_or_id`). The request
  field carrying it is renamed from `id` to `playlist` to signal it now
  accepts either shape — existing callers sending `id` must switch to
  `playlist`.
- Update `README.md`'s documented `POST /api/playlists` contract to match.

Creating a *custom* (non-YouTube-backed) playlist from the SPA is out of
scope — that flow has no URL/ID input to combine and doesn't fit this form.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `playlist-crud`: the Create Playlist requirement's input contract changes
  — the request now carries a `playlist` field (renamed from `id`) that
  accepts either a bare YouTube playlist ID or a full YouTube playlist URL,
  with a new scenario for a malformed/unrecognized value.

## Impact

- `src/domain/shared/playlist_id.rs`: new `PlaylistId::from_url_or_id`.
- `src/http/playlists/dto.rs`: `CreatePlaylistRequest.id` renamed to
  `.playlist`.
- `src/http/playlists/mod.rs`: `create_playlist` handler uses
  `PlaylistId::from_url_or_id` instead of `PlaylistId::new`.
- `openspec/specs/playlist-crud/spec.md`: Create Playlist requirement text
  and scenarios updated.
- `README.md`: `POST /api/playlists` row and example `curl` updated.
- `web/src/api.js`, `web/src/components/`: new create-playlist form and POST
  support.
- Any existing external caller of `POST /api/playlists` sending `{"id": ...}`
  must switch to `{"playlist": ...}`.
