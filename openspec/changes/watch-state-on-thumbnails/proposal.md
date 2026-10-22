## Why

The web UI shows how far a started video has been watched only on "Continue watching" cards. The same video listed in its channel, playlist or category shows nothing. The progress bar is thin and easy to miss, and watched videos look the same as unwatched ones except for a small tick. That makes long lists hard to scan.

## What Changes

- Every video thumbnail (home sections, category view, channel and playlist video lists) shows a progress bar when the video is unwatched and has a saved position, not only "Continue watching" cards.
- The progress bar becomes more visible: it is thicker, its track has more contrast, and the duration badge and watched tick sit above it instead of overlapping it.
- While a video is in progress, the thumbnail's duration badge shows the time left (e.g. `12m left`) instead of the full length.
- Watched videos are de-emphasized: a dimmed thumbnail and a muted title, with the watched tick staying at full strength. On grid cards, hovering restores full strength. In a video list, the selected video's title keeps its highlight.
- The watched tick requirement now names the category view too, which already shows the tick.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`:
  - "Home Page Layout" stops scoping the progress display to "Continue watching".
  - "Watched Tick On Video Thumbnails" also covers the category view.
  - New requirements: progress (bar + time left) on every video thumbnail, and de-emphasis of watched videos.

## Impact

- Web only, no backend or API change: `Video` and `HomeVideo` already carry `watched`, `position_seconds` and `duration_seconds`.
- `web/src/components/`:
  - `VideoGrid.tsx`, `VideoListPane.tsx`, `WatchProgressBar.tsx`, `WatchedTick.tsx`
  - `Home.tsx`: drops the `showProgress` flag
- New pure logic in `web/src/lib/`: the in-progress rule and the time-left format.
- Vitest: `Home.test.tsx` and the channel/playlist/category detail tests.
