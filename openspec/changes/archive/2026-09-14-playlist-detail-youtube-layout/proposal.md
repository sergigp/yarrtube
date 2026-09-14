## Why

The playlist detail view splits its width three ways (video list, player, info panel), so the player is capped at roughly half the viewport even on wide screens, and the info panel repeats the video's title right next to the player where it already draws attention. Reworking the layout to a two-column, YouTube-style arrangement — player big and central, video list as a fixed-width "up next" column on the right — gives the player more room and matches a layout users already know.

## What Changes

- Swap the three-column grid (`video list | player | info`) for a two-column one: a main column (title + player + info, stacked) and a fixed-width (~360px) right column holding the video list, mirroring YouTube's watch-page proportions.
- When a video is selected, show its title above the player as a heading (not just inside the info panel below).
- Move the info panel below the player instead of beside it:
  - Drop the Title, Created at, and Updated at fields.
  - Show Status and Quality as always-visible chips (reusing the existing `.status-badge`/`.chip-group` classes already used in `PlaylistList.jsx` and `TasksView.jsx`), instead of `<dl>` rows.
  - Merge the Filename and Playlist path fields into a single combined path field (e.g. `/watch_later/example_video.mp4`); when the video has no filename yet, show just the playlist's base path.
  - Keep the Delete button (custom playlists only).
- On narrow screens (<900px), drop the off-canvas drawer for the video list. Everything stacks in one scrollable column instead — title, player, info, then the video list last (true YouTube-mobile placement) — so the "Videos" toggle button, its open/close state, and the drawer/backdrop CSS are all removed as dead code.

## Capabilities

### New Capabilities
_None._

### Modified Capabilities
_None._ This is a pure frontend layout/presentation rework of the existing playlist detail view — no HTTP endpoint, request/response, or other backend behavior changes. Every existing spec in this repo (`web-ui`, `video-listing`, `video-playback`, etc.) describes server/HTTP-level guarantees only, never frontend layout — consistent with how the original `add-web-ui` change kept view/layout decisions (which views exist, polling interval, no router library) out of specs and in `design.md`/the proposal instead. `skip_specs: true` is set in `.openspec.yaml` accordingly.

## Impact

- `web/src/components/PlaylistDetail.jsx`: `VideoDetail` component's rendered fields change (drop Title/Created at/Updated at, add chips, merge path); layout markup reorders (title above player, info below); `sidebarOpen` state, the "Videos" toggle button, and the drawer/backdrop divs are removed.
- `web/src/App.css`: `.playlist-detail-layout` grid changes from three columns to two (main column + fixed ~360px right column); `.sidebar-toggle`, `.video-sidebar` off-canvas/backdrop rules, and the `@media (max-width: 900px)` drawer behavior are replaced with a single-column stacking rule (video list last); new styles for the title-above-player heading and the merged path field.
- No API, database, or CLI changes.
