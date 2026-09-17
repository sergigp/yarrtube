## Why

The current SPA organizes browsing around three flat tabs (Playlists, Channels, Tasks) with no real routing, a hand-rolled deep-link mechanism for jumping into a specific video, two near-duplicate "add" forms that both require the user to type a filesystem path, and a video detail view where the whole page scrolls and available actions are hidden behind a "⋯" menu. A round of user feedback identified concrete friction across navigation, the add forms, the home page, and the detail views; this change addresses that feedback as one coherent UX pass rather than a series of disconnected tweaks, since several items depend on the same underlying navigation model.

## What Changes

- The add dialog's "Advanced options" gain a **Path** field for both modes, pre-filled client-side from a filesystem-safe slug of the playlist name (`playlists/<slug>`) or the entered channel handle/URL (`channels/<slug>`), kept in sync automatically until the user edits it directly. The field stays editable and is still submitted as `path` on `POST /api/playlists` / `POST /api/channels` — **the API contract is unchanged**, this is a frontend-only default.
- Add a real router (`react-router-dom`) with routes for `/`, `/playlists/:id`, `/channels/:id`, `/tasks`; the hand-rolled `deepLink` prop-drilling is replaced by a `?video=<id>` query parameter.
- Replace the Home tab's video list with a card grid (thumbnail + title) and add a right-hand sidebar listing channel and playlist titles; clicking a sidebar entry navigates to that channel's/playlist's detail route.
- Remove the standalone Playlists and Channels tabs/list views; browsing playlists and channels now happens from the Home sidebar.
- Merge the "Add Playlist" and "Add Channel" dialogs into a single dialog with a Playlist/Channel switcher, one "Add" button in the header, and a collapsed "Advanced options" section holding quality (renamed "Video quality" with an explanatory tooltip), the path field described above, and, for channels, video limit, whose default drops from 10 to 3. If a create request fails because the path is already in use, the dialog auto-expands Advanced options so the user can see and fix it.
- The add dialog no longer closes on an outside click (only via Escape or its close button).
- In playlist/channel detail views: remove the "back to playlist/channel list" link (that intermediate view no longer exists), make the video player and detail pane fixed on the page so only the video-list sidebar scrolls, enlarge video-list thumbnails and row height so titles can wrap, replace the hidden "⋯" actions menu with visible Reconcile/Delete buttons, and add a link to the video on YouTube.
- Clicking the Yarrtube logo navigates to Home from anywhere in the app.

## Capabilities

### New Capabilities
(none — this change modifies existing capabilities)

### Modified Capabilities
- `web-ui`: navigation model (real routes replacing tab state), Home layout (card grid + sidebar), detail view layout (fixed video pane, visible actions, YouTube link), merged add dialog behavior (auto-filled-but-editable path under advanced options, tooltip, outside-click-safe)

## Impact

- **Frontend** (`web/src`): new `react-router-dom` and `@radix-ui/react-tooltip` dependencies; `App.jsx` restructured around routes; `Home.jsx`, `PlaylistDetail.jsx`, `ChannelDetail.jsx`, `AddPlaylistDialog.jsx`/`AddChannelDialog.jsx`/`CreatePlaylistForm.jsx`/`CreateChannelForm.jsx` rewritten or merged; `PlaylistList.jsx`/`ChannelList.jsx` retired as standalone pages (their list rendering moves into the Home sidebar); `App.css` gains card-grid, fixed-detail-layout, and tooltip styles; a new client-side slugify helper drives the path field's auto-fill.
- **Backend** (`src`): unchanged — `POST /api/playlists` and `POST /api/channels` keep accepting `path` exactly as they do today.
- No data migration.
