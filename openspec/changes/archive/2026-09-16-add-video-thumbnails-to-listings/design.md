## Context

`Video.thumbnail_filename` and each `VideoResponse` already carry a
video's thumbnail filename (from the `video-thumbnails` change). Building a
usable media URL from a filename requires the container's storage `path`
(`web/src/api.js`'s `videoMediaUrl(path, filename)`), which `PlaylistDetail`/
`ChannelDetail` already have via their `playlist`/`channel` prop. The `Home`
tab and `/videos/recent` have no such object in hand — `RecentVideo`'s
`VideoSource` only carries the tracking playlist/channel's id today. See
proposal.md - Why.

## Goals / Non-Goals

**Goals:**
- Let the SPA build a thumbnail media URL for every row in `Home` and the
  playlist/channel sidebar lists with no extra per-row network round-trip.
- Keep unthumbnailed rows visually aligned with thumbnailed ones.

**Non-Goals:**
- No new HTTP routes or media-serving changes (thumbnails are already
  served by the existing `/media` mount).
- No backfill or thumbnail generation — this only surfaces what's already
  recorded.

## Decisions

- **Thread the source's storage `path` through `VideoSource`, not a computed
  `thumbnail_url`.** `VideoSource::Playlist`/`Channel` become
  `Playlist(PlaylistId, PlaylistPath)` / `Channel(ChannelHandle, PlaylistPath)`.
  `VideoSearcher::list_recent` already loads the full `Playlist`/`Channel`
  per source to get its id — `.path` is already in hand there, so this is a
  zero-extra-query addition. Alternative considered: have the backend return
  a fully-built `thumbnail_url` string. Rejected because every other
  endpoint hands the client a bare `path`/`filename` pair and lets
  `videoMediaUrl` build the URL — a one-off `thumbnail_url` field would be
  an inconsistent shape for the same underlying data, and duplicates URL
  logic across backend and frontend.
- **Blank placeholder box for a missing thumbnail, not a hidden slot.** A
  fixed-size empty box keeps every row's title starting at the same
  x-offset in both `Home` and the sidebar lists, matching the "the layout is
  as important as the pixel" instinct behind the rest of the list styling.

## Risks / Trade-offs

- [`VideoSource`'s shape change touches every match on it] -> confined to
  `video_searcher.rs` (construction) and `dto.rs` (mapping to
  `RecentVideoSourceResponse`); no other code matches on it today.
