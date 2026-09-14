## 1. Grid layout: two columns instead of three

- [x] 1.1 In `web/src/App.css`, replace `.playlist-detail-layout`'s three-column grid (`280px minmax(0, 2fr) minmax(0, 1fr)`) with two columns: a flexible main column and a fixed ~360px right column for the video list; verify by running `npm run dev` and confirming the player fills the main column width at a desktop viewport (≥900px).
- [x] 1.2 Reorder the DOM/CSS so the main column (title + player + info) renders before the video list column, and the video list column is pinned to the right at desktop widths; verify visually in the dev server.

## 2. Title above the player

- [x] 2.1 In `web/src/components/PlaylistDetail.jsx`, render the selected video's title as a heading above the player, only when a video is selected; verify by selecting a video in the dev server and confirming the title appears above the player and disappears when no video is selected.

## 3. Info panel rework

- [x] 3.1 In `VideoDetail` (`web/src/components/PlaylistDetail.jsx`), remove the Title, Created at, and Updated at fields from the rendered output; verify by inspecting the rendered info panel for a selected video.
- [x] 3.2 Render Status and Quality as always-visible chips using the existing `.status-badge`/`.chip-group` classes (same pattern as `PlaylistList.jsx`/`TasksView.jsx`), replacing their `<dl>` rows; verify chips render for videos in every status (e.g. `DOWNLOADED`, `PENDING`, `IN_PROGRESS`, `ERRORED`) and when quality is absent.
- [x] 3.3 Replace the separate Filename and Playlist path fields with a single combined path field (`${playlist.path}/${video.filename}` when a filename exists, otherwise just `playlist.path`); verify against a downloaded video (full path shown) and a not-yet-downloaded video (base path only, no trailing slash or placeholder filename).
- [x] 3.4 Move the info panel's markup below the player in the main column (instead of beside it in its own grid column); verify visually that it appears directly under the player, above/around the Delete button.

## 4. Mobile: drop the drawer, stack inline

- [x] 4.1 In `web/src/components/PlaylistDetail.jsx`, remove the `sidebarOpen` state, the "Videos" toggle button, and the backdrop `div`; verify with a repo-wide search that no remaining code references `sidebarOpen` or the toggle.
- [x] 4.2 In `web/src/App.css`, remove the `.sidebar-toggle`, `.video-sidebar-backdrop`, and off-canvas/drawer transform rules under `@media (max-width: 900px)`, replacing them with a single-column stacking rule that orders the video list last (after title, player, and info); verify at a narrow viewport (<900px) in the dev server that the video list appears at the bottom of the scrollable page with no drawer/toggle present.

## 5. Final check

- [x] 5.1 Run `npm run lint` (oxlint) in `web/` and confirm it passes.
- [x] 5.2 Run `npm run dev`, exercise the playlist detail view at both a desktop (≥900px) and a mobile (<900px) width, and confirm: video list on the right (desktop) / stacked last (mobile); title shown above the player when a video is selected; info panel below the player with chips and the merged path field; Delete button still works for a custom playlist's video.
