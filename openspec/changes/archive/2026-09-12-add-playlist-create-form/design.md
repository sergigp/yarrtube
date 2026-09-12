## Context

See `proposal.md` - Why. Two existing precedents shape this design:

- `VideoId::from_url_or_id` (`src/domain/shared/video_id.rs`) already parses
  a bare video ID or a full/short YouTube watch URL, used server-side by the
  add-video-to-custom-playlist endpoint. `PlaylistId` currently has no such
  parser — `PlaylistId::new` only rejects empty values.
- The SPA (`web/src`) is currently read-only: `usePolling` wraps GET-only
  `api.js` functions. This change adds the SPA's first mutating request.

## Goals / Non-Goals

**Goals:**
- Accept a bare YouTube playlist ID or a full YouTube playlist URL in one
  input, both in the HTTP API and the SPA form.
- Keep the parsing symmetric with `VideoId::from_url_or_id` so the two
  identifier types stay consistent for future readers.

**Non-Goals:**
- Custom playlist creation from the SPA (see proposal.md).
- Any change to `custom-playlist-crud` or its video-add flow.
- Client-side validation beyond what's needed for basic UX (empty-field
  checks); authoritative validation stays server-side as it already is for
  path/quality/name.

## Decisions

- **`PlaylistId::from_url_or_id` mirrors `VideoId::from_url_or_id`'s shape**:
  non-URL input is treated as a bare ID; `https://`/`http://` input is
  matched against `youtube.com` / `www.youtube.com` / `m.youtube.com` hosts
  and the `list` query parameter is extracted. Unlike `VideoId`, there is no
  `youtu.be`-style short host for playlists, so only the `youtube.com`
  family is recognized. Accepting `list=` on any path (not just
  `/playlist`) lets a "watch a video, then add its whole playlist" URL work
  too — this is the most common way playlist URLs actually get copied.
  Alternative considered: require the path to be exactly `/playlist`. Rejected
  because it would silently fail on the equally common `watch?v=...&list=...`
  form.
- **Request field renamed `id` -> `playlist`**: see proposal.md's BREAKING
  note. Chosen over keeping `id` because `id` reads as "the identifier",
  which is misleading once the field also accepts a URL; `AddVideoRequest`
  already established `video` as the naming convention for an ID-or-URL
  field on this codebase.
- **Parsing happens in the domain layer, not the HTTP layer or the SPA**:
  keeps `PlaylistId` the single owner of what a valid playlist identifier
  looks like (consistent with `rust-architect`'s layering), and means any
  future caller of the HTTP API (CLI, another client) gets URL support for
  free rather than needing to duplicate the parsing.
- **SPA form is a new `CreatePlaylistForm` component**, plain controlled
  inputs (no form library — the codebase has no form library and this is
  four fields), calling a new `createPlaylist` in `api.js`. Placed above
  `PlaylistList` in `PlaylistsTab`. On success, the form clears; the list
  picks up the new playlist on its next poll tick (consistent with how the
  rest of the SPA already treats staleness — no new refetch mechanism
  introduced).

## Risks / Trade-offs

- [Breaking the documented `id` field] -> Acceptable per proposal.md: this
  is a personal single-operator project and the user confirmed the rename
  is fine; `README.md` is updated in the same change so the documented
  contract never goes stale.
- [Accepting `list=` on any YouTube path, not just `/playlist`, could in
  theory extract an ID from a URL the user didn't intend as a playlist
  reference] -> Low risk: the extracted ID is still verified against the
  YouTube Data API before persisting (existing "Nonexistent YouTube
  playlist" behavior), so a wrong extraction just surfaces as a 400 rather
  than persisting bad data.

## Migration Plan

Single-PR change, no data migration (no stored data shape changes — only
the HTTP request field name and an added parser). Deploy is just shipping
the new binary and SPA build together, as usual for this project (no
rolling/mixed-version concern since it's a single daemon instance).
