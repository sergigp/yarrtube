## Why

Downloaded videos can currently only be confirmed by filename in the video list — there's no way to actually watch one from the SPA. The daemon already knows the exact on-disk path of every downloaded file (playlist path + recorded filename under the configured videos root), and this deployment is self-hosted on a personal NAS with no authentication boundary to protect, so those files can be exposed for direct in-browser playback with minimal new code: a static file mount rather than a bespoke domain-aware endpoint.

## What Changes

- Mount the daemon's configured videos root directory as a static file server at `/media`, so a downloaded video's on-disk path is directly fetchable over HTTP (e.g. `/media/<playlist-path>/<filename>`).
- Add a video detail view to the SPA: selecting a video from a playlist's list shows its full metadata (title, status, quality, filename, playlist path, created/updated timestamps) and, when the video is downloaded, an inline `<video>` player pointed at its `/media/...` URL.
- No new backend domain code, no new HTTP handler, no new persisted state, no authentication — this reuses data the daemon already exposes via `list_videos_for_playlist` and the playlist's already-known path.

## Capabilities

### New Capabilities
- `video-playback`: HTTP-level static serving of downloaded video files from the configured videos root, for in-browser playback.

### Modified Capabilities
(none — `video-listing` already returns every field the detail view needs; `web-ui` already covers serving the SPA shell and its own assets and is unaffected by mounting a separate, unrelated path)

## Impact

- `serve.rs`: nest a static file service (`tower-http`'s `ServeDir`) at `/media`, rooted at the existing configured videos path.
- `Cargo.toml`: new dependency, `tower-http` (`fs` feature), for range-request-capable static serving.
- `web/src/components/PlaylistDetail.jsx`: add selected-video state, a detail panel, and a `<video>` element for downloaded videos.
- `web/src/api.js`: helper to build a video's `/media/...` URL from the playlist's `path` and the video's `filename`.
- No changes to existing HTTP routes, domain services, or the database schema.
