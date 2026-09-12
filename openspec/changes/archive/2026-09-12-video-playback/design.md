## Context

See proposal.md - Why. The daemon already resolves a video's on-disk path the same way in three places in `VideoService` (`videos_path.join(playlist.path.as_str())`, then the recorded `filename`), and `PlaylistPath` already rejects `..` segments, absolute paths, and filesystem-unsafe characters at construction time. `web-ui` already serves the SPA itself from the same daemon with no authentication. This deployment target is a personal NAS with a single trusted network, per the user's explicit call in exploration.

## Goals / Non-Goals

**Goals:**
- Make a downloaded video's bytes fetchable over HTTP with the least possible new backend code.
- Support seeking (HTTP range requests) so the browser's native `<video>` player works well.
- Show full video/playlist metadata when a video is selected in the SPA, using data already returned by existing endpoints.

**Non-Goals:**
- Authentication or per-video access control (matches the rest of the app; see proposal.md - Why).
- A domain-aware "does this file belong to a real, downloaded video" check at request time - out of scope for this change; see Risks below.
- Thumbnails, transcoding, adaptive bitrate, or any media processing beyond serving the file as downloaded.

## Decisions

- **Static directory mount over a domain-aware handler.** Nest `tower-http`'s `ServeDir`, rooted at the existing `videos_path` config value, at `/media` in `serve.rs`, alongside the existing SPA router. Rejected alternative: a hand-written `GET /api/playlists/{id}/videos/{video_id}/file` handler that looks up the video via `VideoRepository::find`, checks `status == Downloaded`, and streams the file. That alternative gives a "only serves files for real, tracked, downloaded videos" guarantee and a clean 404 for anything else, at the cost of hand-rolling HTTP range parsing. Chosen because: this is a personal, unauthenticated, single-user NAS tool (decided in exploration), `ServeDir` gets range support and path-traversal protection for free, and the SPA already has every field (`playlist.path`, `video.filename`) needed to construct the media URL client-side without a new domain-aware endpoint.
- **Mount path: `/media`.** Chosen over nesting under `/api` because it's not an API response - it's raw file bytes - and a flat top-level path keeps the URL construction in the frontend simple: `` `/media/${playlist.path}/${video.filename}` ``.
- **URL constructed client-side, not returned by the API.** `VideoResponse` already has `filename`; `PlaylistDetail.jsx` already has `playlist.path` in scope. Adding a computed media URL field to the API response would duplicate data already present, so the frontend builds it directly (mirrors how the backend itself computes the same join in `VideoService`).
- **Detail panel is pure frontend state.** No new backend fields are needed - `title`, `status`, `quality`, `filename`, `created_at`, `updated_at` are already in `VideoResponse`, and `path`, `name` are already in `PlaylistResponse`/the `playlist` prop already passed into `PlaylistDetail`.

## Risks / Trade-offs

- **[Risk]** `ServeDir` serves anything under `videos_path`, not just files that correspond to a tracked, downloaded `Video` record - e.g. a stray or partially-written file, or a file for a video whose record has since been deleted. → **Mitigation**: acceptable for a personal, single-user NAS deployment with no auth boundary elsewhere in the app either; the SPA never links to a URL it wasn't given a `filename` for, so this is only reachable by a client constructing URLs by hand.
- **[Risk]** Mounting `/media` before (or after) the SPA's fallback route could shadow one or the other if ordering is wrong. → **Mitigation**: register the `/media` nested service before the SPA's catch-all fallback in the router, and add a test asserting both a known media path and the SPA root still resolve correctly.
- **[Risk]** `tower-http` is a new dependency this project didn't previously need. → **Mitigation**: it's a widely-used, actively maintained companion crate to `axum` (same ecosystem, same maintainers); only the `fs` feature is needed.

## Migration Plan

1. Add `tower-http` (`fs` feature) to `Cargo.toml`.
2. In `serve.rs`, nest a `ServeDir::new(videos_path)` at `/media`, registered before the existing SPA fallback.
3. Add `web/src/api.js` helper to build a video's media URL from `playlist.path` + `video.filename`.
4. Add selected-video state and a detail panel (with inline `<video>` for downloaded videos) to `PlaylistDetail.jsx`.

No database migration, no breaking change, no rollback complexity beyond removing the mount and UI addition.
