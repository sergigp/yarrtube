## Why

The sidebar lists channels and playlists in subscription order (`ORDER BY rowid`) and shows every row. With many subscriptions it turns into a long, unordered list where the channels with new videos get lost. The unwatched badges behind it are also expensive: every 3s poll runs one query per stored channel video, and it keeps polling in background tabs and fetches the same list twice.

## What Changes

- `GET /api/channels` and `GET /api/playlists` return entries sorted alphabetically by name, case-insensitive, instead of in insertion order.
- The channel unwatched counts are computed from 3 bulk reads (channels, channel videos, their videos) instead of one lookup per video. The API response is unchanged.
- Sidebar channels section:
  - Channels are ordered in two groups: those with unwatched videos first, by unwatched count (most first) with ties by name, then caught-up channels alphabetically.
  - While collapsed, the section shows the unread channels, up to 10. The rest keep the same order behind a "Show N more" control.
  - When no channel has unwatched videos, the first 5 channels are shown before the collapse.
- Sidebar playlists section: alphabetical, with the first 5 shown before the collapse.
- The active channel or playlist is always visible in the sidebar, even when it would otherwise be collapsed.
- Each section's expanded/collapsed state is remembered per browser.
- When channels and playlists together number more than 15, a single search box at the top of the sidebar filters both sections by name, case-insensitive. While it has text, collapse is ignored and sections with no matches are hidden.
- Data refresh:
  - Polling pauses while the tab is hidden and refetches when it becomes visible again.
  - The sidebar's channel and playlist lists refresh every 60s. Views that show live progress (tasks, video lists) keep a 3s interval.
  - The channel list is refetched right after actions that change it: mark watched, sync, delete, add, and a playback progress report that changes the video's watched state. The progress endpoint now returns `{ "watched": true|false }` so the client can tell; reports that leave it unchanged trigger no refetch.
  - The sidebar and detail views share one fetch of the channel list and one of the playlist list.
- Out of scope: unwatched badges and "mark watched" for playlists, and server push (SSE/WebSockets).

## Capabilities

### New Capabilities

### Modified Capabilities
- `channel-crud`: listed channels are sorted alphabetically by name.
- `playlist-crud`: listed playlists are sorted alphabetically by name.
- `watch-state`: recording playback progress responds with the video's resulting watched state.
- `web-ui`: sidebar grouping, ordering, collapsing, active-entry visibility, remembered collapse state and search; badge refresh after the user's own actions; data refresh pauses in hidden tabs.

## Impact

- **Backend (Rust)**: the channel and playlist repositories' `list` ordering, a new `ChannelVideoRepository::list`, and `ChannelViewSearcher` counting unwatched videos in Rust from bulk reads instead of its per-video lookups. `POST /api/videos/<id>/progress` responds `200 { "watched": bool }` instead of `204`, and `VideoWatchStateUpdater::update` returns the resulting watched state. There's no schema migration.
- **Frontend**: adds the `@tanstack/react-query` dependency. It replaces `usePolling` for the channel and playlist lists (and the other polled fetches, so they all pause in hidden tabs). Affects `Sidebar.jsx`, `ChannelDetail.jsx`, `PlaylistDetail.jsx`, `Home.jsx`, `TasksView.jsx`, the watch-progress hook, the add dialog and `main.jsx`.
- **Smoke tests**: the channel and playlist specs locate the first sidebar row. They should still pass with one tracked entry, but any sidebar lookup must still find a row that may now be collapsed or reordered. The channel and playlist specs expect `200` from the progress report.
