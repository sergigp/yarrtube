## Why

The web UI is hard to use on a phone, which is the main way it gets watched. The app shell never lets the document scroll, so iOS Safari can't run content under its bars. Sidebar actions only appear on hover. On the detail page, nested scroll areas cut off the video details and squeeze the title. Fixing this now makes the NAS-hosted app usable from an iPhone.

## What Changes

- On mobile, the document scrolls instead of an inner container. The header is sticky, and on detail views the player is sticky just below it. The video details and the video list scroll with the page. The desktop layout is unchanged.
- The mobile player has no minimum-height wrapper, so there's no empty space above or below the video. The page padding above the player is reduced on mobile.
- The video detail pane shows the title on its own full-width row. The status and quality chips, the file path and "Open on YouTube" go into a collapsible section. It's collapsed by default on mobile and expanded on desktop.
- Sidebar row actions (Sync, Mark all watched, Delete) move from hover-only buttons into a `...` menu that's always visible on each row. This also frees the row width the invisible buttons used to take. The mobile drawer gets narrower (`w-64`).
- While the mobile drawer is open, the page behind it can't scroll.
- On home page cards, the channel avatar and a new channel name link to the channel page. The rest of the card still links to the video. On the channel detail pane, the avatar links to the channel page.
- Playlist and channel detail views get a page header with the name (plus avatar for channels), an "N videos · M unwatched" summary, and Sync, Mark all watched (channels only) and Delete actions. On phones the actions show as icons only. The channel view's "Mark all watched" button moves into this header.
- The sidebar's "Channels" and "Playlists" titles get a heading style, so they no longer look like list rows.
- The recently-synced videos endpoint now includes each source's display name (channel name or playlist name), so home cards can show it without another request.
- The default Vite favicon is replaced with a Yarrtube icon: a green rounded square with a white "Y". We also add an `apple-touch-icon`, a `theme-color` and a minimal web app manifest (`display: standalone`), so the app can be added to the iOS home screen.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `video-listing`: "List Recently Synced Videos Across Sources" now includes the source's name
- `web-ui`:
  - modified: "Detail View Fixed Video Area" (mobile now uses a sticky player and page scrolling), "Sidebar Row Actions" (a `...` menu replaces hover buttons), "Collapsible Mobile Sidebar" (the page behind the open drawer is locked instead of "the document never scrolls"), "Home Page Layout" (channel avatar and name link to the channel), "Mark Channel Watched From Channel View" (the control moves into the page header)
  - added: collapsible video details with a prominent title, channel avatar links, compact mobile player, app icon and manifest, detail view page header, sidebar section headings

## Impact

- Backend: `VideoSource` (domain) and `VideoSearcher::list_recent` carry the source name, and `RecentVideoSourceResponse` gains a `name` field. This only adds a field, so existing clients keep working. No schema or repository changes.
- `web/index.html`, `web/public/` (favicon, touch icon, manifest)
- `web/src/App.jsx` (shell scrolling model)
- `web/src/components/Sidebar.jsx`
- `web/src/components/Home.jsx`
- `web/src/components/PlaylistDetail.jsx`
- `web/src/components/ChannelDetail.jsx`
- `web/src/components/DetailHeader.jsx` (new), `web/src/components/VideoPlayer.jsx`
- Behaviour on iOS Safari must be checked on a real device. A desktop browser emulating a phone doesn't reproduce how Safari handles its toolbars.
