## Why

Custom playlists (playlists with no YouTube playlist behind them, curated by
adding/removing individual videos) were built but never wired into the web
UI beyond a single delete button, and the README still lists the feature as
"coming soon." No real custom playlists exist in the running instance. The
code adds a second playlist kind that threads through the domain, HTTP, and
reconciliation layers for a feature that isn't in active use — removing it
simplifies the codebase back to a single playlist shape.

## What Changes

- **BREAKING**: Remove the create-custom-playlist, add-video-to-custom-playlist,
  and remove-video-from-custom-playlist HTTP endpoints
  (`POST /custom-playlists`, `POST /custom-playlists/{id}/videos`,
  `DELETE /custom-playlists/{id}/videos/{video_id}`)
- Remove `CustomPlaylistVideoAdder` and `CustomPlaylistVideoRemover` domain
  services, and `PlaylistCreator::create_custom`
- Remove `CreateCustomPlaylistError`, `AddVideoToCustomPlaylistError`, and
  `RemoveVideoFromPlaylistError::NotCustomPlaylist`
- Collapse `PlaylistKind` to a single `YoutubeLinked` variant, dropping
  `Custom`; `Playlist.kind` and the `kind` column/API field are kept as-is
  so the shape doesn't need to change again if this is reintroduced later
- Simplify `VideoReconciler` to always run the YouTube-linked reconcile path
  (drops the now-dead kind branch)
- Remove the web UI's per-video Delete button and its `deleteVideoFromCustomPlaylist`
  API call, both gated on `kind === 'custom'` and unreachable without the
  removed create-custom endpoint
- Remove the two "Coming soon: custom playlists..." lines from README.md

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `playlist-crud`: drop the requirements and scenarios describing behavior
  that varies by playlist kind ("of either kind", "regardless of kind",
  playlists of both kinds in one list, path-uniqueness across kinds) since
  only one kind exists going forward

### Removed Capabilities
- `custom-playlist-crud`: the entire capability is removed — creating a
  custom playlist and adding/removing videos on one are no longer supported

## Impact

- **API**: removes the three `/custom-playlists/*` endpoints (breaking for
  any caller using them)
- **Domain**: `src/domain/playlist/` (`PlaylistKind`, errors),
  `src/domain/services/` (adder/remover services, `PlaylistCreator`),
  `src/domain/video/errors.rs`
- **HTTP**: `src/application/http/custom_playlists/` (deleted),
  `src/application/http/mod.rs` (routes, `AppState`),
  `src/application/http/playlists/dto.rs` (unaffected — `kind` field stays)
- **Reconciliation**: `src/domain/services/video_reconciler.rs`
- **Web UI**: `web/src/api.js`, `web/src/components/PlaylistDetail.jsx`
- **Docs**: `README.md`
- **Specs**: `openspec/specs/custom-playlist-crud/` removed,
  `openspec/specs/playlist-crud/spec.md` trimmed
- **Data**: no schema change — the `kind` column stays and keeps storing
  `youtube_linked`; no existing rows use `custom`
