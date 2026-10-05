## Why

Users want to watch videos faster than real time. The player uses the browser's native controls, which offer no 1.1x speed in any browser and no speed menu at all in Firefox (and inline iOS Safari).

## What Changes

- Add a playback speed control to the title row of the selected video's detail pane, next to the "⋮" actions menu, offering 1x, 1.1x, 1.25x, 1.5x and 2x.
- The chosen speed applies only to the video currently playing: selecting another video, or reloading the page, starts it at 1x. Nothing is remembered.
- The control reflects speed changes made through the browser's own player menu, including speeds outside the offered list.
- The control is disabled while the selected video has not finished downloading.
- No player library is introduced; the native `<video controls>` element stays.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `web-ui`: adds a playback speed control to the playlist/channel detail view's video detail pane.

## Impact

- Frontend only (`web/`): the video detail pane, the playlist and channel detail views that own the `<video>` element, and a new hook/component for the speed.
- No backend, API, DTO or storage changes; no new dependencies.
