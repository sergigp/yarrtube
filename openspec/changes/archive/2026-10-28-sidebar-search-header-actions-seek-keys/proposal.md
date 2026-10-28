# Proposal

## Why

The sidebar search sat between Categories and Channels and ignored categories, so it couldn't find one; detail view page headers hid most actions behind a "⋮" even on desktop, where there is room for all of them; and the player had no keyboard shortcut to skip back or forward by a useful step.

## What Changes

- Sidebar search moves above the Categories section, counts categories toward its "more than 15" threshold and filters categories like channels and playlists (a section with no match hides; "nothing matches" covers all three). Its accessible name becomes "Search categories, channels and playlists".
- Channel, playlist and category page headers: at or above the desktop breakpoint every action (sync, manage sources, mark all watched, home and unwatched count settings, edit settings, delete) is a square icon-only button with a tooltip naming it, and there is no "⋮". Below it, only the "⋮" shows, and it holds every action, sync included. **BREAKING** for anything relying on the "⋮" on desktop or the visible sync button on mobile.
- New: while a downloaded video is in a detail view's player, ← / → (outside text fields, without modifiers) seek it 10 s back / forward, replacing the browser's own step, with YouTube-style feedback over that side of the player: chevrons and the seconds sought, adding up across quick presses, fading on its own.

## Capabilities

### New Capabilities

### Modified Capabilities
- `web-ui`: Sidebar Search, Sidebar Categories Section, Detail View Page Header, Category View, Mark Channel Watched From Channel View and Hide Playlist Unwatched Count change; a new Player Keyboard Seeking requirement is added.
- `e2e-tests`: Search Coverage and Mobile Navigation Coverage change; new Page Header Actions Coverage and Keyboard Seeking Coverage requirements.

## Impact

- Web UI only (`web/src`): `Sidebar`, `SidebarCategories`, `sidebarSections`, `DetailHeader`, `CategoryDetail`, `VideoPlayer`, new `useSeekKeys` hook and `seekKeys` lib, a test viewport switch in `src/test/`.
- e2e specs: `search`, `seekKeys`, `detailHeader`, `mobileLayout` and those that opened header actions through the "⋮".
- No API or backend change.
