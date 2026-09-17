## 1. Frontend: routing foundation

- [x] 1.1 Add `react-router-dom` and `@radix-ui/react-tooltip` to `web/package.json`, verify `npm install` succeeds
- [x] 1.2 Verify (and if needed adjust) the daemon's static-file serving so a direct load of `/playlists/:id` or `/channels/:id` falls back to serving `index.html` the same way `/` does today; verify by starting the built binary and curling a non-root, non-asset path
- [x] 1.3 Restructure `App.jsx` around a `BrowserRouter` with routes for `/`, `/playlists/:id`, `/channels/:id`, `/tasks`; remove the `tab` state and the `PlaylistsTab`/`ChannelsTab`/`deepLink` prop-drilling; verify by manually navigating each route and confirming the URL updates and browser back/forward work
- [x] 1.4 Make the logo a `<Link to="/">`; verify clicking it from a detail view and from the tasks view returns to home

## 2. Frontend: Home page

- [x] 2.1 Rework `Home.jsx`'s recent-videos list into a card grid (thumbnail + title below), replacing the `<ul className="list">` rendering; add corresponding grid CSS in `App.css`
- [x] 2.2 Add a right-hand sidebar to `Home.jsx` with "Channels" and "Playlists" sections, each listing tracked titles only (reuse `fetchChannels`/`fetchPlaylists`); clicking an entry navigates to `/channels/:id` or `/playlists/:id`
- [x] 2.3 Retire `PlaylistList.jsx`/`ChannelList.jsx` as standalone routed pages (delete the files if nothing else references them after 2.2, or confirm no remaining import)
- [x] 2.4 Make a home video card navigate to `/playlists/:id?video=<id>` or `/channels/:id?video=<id>` depending on its source kind
- [x] 2.5 Manually verify: with synced videos, tracked playlists, and tracked channels present, Home renders the grid and both sidebar sections, and every click target navigates correctly

## 3. Frontend: merged Add dialog

- [x] 3.1 Create a single `AddDialog` component with a Playlist/Channel switcher, replacing `AddPlaylistDialog`/`AddChannelDialog`; wire it to one always-visible "Add" button in the header (remove the per-tab conditional Add Playlist/Add Channel buttons)
- [x] 3.2 Merge `CreatePlaylistForm`/`CreateChannelForm` into mode-driven forms rendered inside `AddDialog`; wrap path, quality (playlist) / path, quality, video limit (channel) in a collapsed-by-default "Advanced options" disclosure
- [x] 3.3 Add a `slugify(value)` JS helper (`web/src`, e.g. alongside `formatDateTime.js`) — lowercase, non-alphanumeric runs collapsed to `-`, trimmed; unit-test-equivalent manual check for empty/whitespace/unicode input
- [x] 3.4 Wire the path field to auto-fill: playlist mode from `playlists/${slugify(name)}` on every Name keystroke; channel mode from `channels/${slugify(handle)}` (stripping a leading `@`, best-effort extracting the last URL segment if the value looks like a URL) on every Channel field keystroke; stop auto-filling once the user edits the path field directly (reset on dialog close/reopen or mode switch)
- [x] 3.5 Rename the quality field label to "Video quality" and add a `@radix-ui/react-tooltip` explaining it controls download resolution and that lower resolutions save storage
- [x] 3.6 Change the channel form's video limit default from 10 to 3
- [x] 3.7 Set `onPointerDownOutside`/`onInteractOutside` on the dialog content to `preventDefault()` so an outside click no longer closes it; verify Escape and the × close control still work
- [x] 3.8 On a create request failing with a path-already-in-use error, expand "Advanced options" (if collapsed) so the path field and error are visible
- [x] 3.9 Manually verify: open the dialog, switch modes, confirm the path field auto-fills and updates live from the name/handle, confirm editing the path stops further auto-fill, submit a playlist and a channel successfully, confirm the video limit defaults to 3, confirm a colliding path both surfaces the existing server error and auto-expands Advanced options, and confirm clicking outside the dialog does not close it

## 4. Frontend: detail view layout

- [x] 4.1 Update `PlaylistDetail.jsx`/`ChannelDetail.jsx` to read the selected video from `?video=` (via `useSearchParams`) instead of the `initialVideoId` prop; remove the now-unused prop plumbing
- [x] 4.2 Remove the "← Back to playlists/channels" link from both detail views
- [x] 4.3 Rework `.playlist-detail-layout`/related CSS so the route's content is bounded to the viewport height with `overflow: hidden`, and only `.video-sidebar` scrolls (`overflow-y: auto`); verify by scrolling a long video list and confirming the player/detail pane don't move
- [x] 4.4 Increase video-list thumbnail size and row height in `App.css` so titles can wrap to multiple lines
- [x] 4.5 Replace `PlaylistActionsMenu`/`ChannelActionsMenu`'s Radix `DropdownMenu` with visible Reconcile/Delete buttons in the detail header toolbar, keeping the existing `ConfirmDialog` delete flow
- [x] 4.6 Add an "Open on YouTube" link in `VideoDetail` (both playlist and channel variants) pointing to `https://www.youtube.com/watch?v=<video.id>`, `target="_blank" rel="noopener"`
- [x] 4.7 Manually verify: open a playlist detail and a channel detail, confirm scrolling the video list leaves the player fixed, confirm Reconcile/Delete are visible without opening a menu, and confirm the YouTube link opens the right video

## 5. Verification

- [x] 5.1 Run `cargo test --locked` and confirm the full backend suite still passes unchanged (no backend behavior was touched by this change)
- [x] 5.2 Run `npm run lint` in `web/` and confirm it's clean
- [x] 5.3 Full manual walkthrough: create a playlist and a channel via the merged dialog (using the auto-filled path both as-is and after editing it), confirm both appear in the Home sidebar and grid, navigate into each detail view, reconcile and delete one of them, and confirm the logo returns to Home from every view
