# Proposal

## Why

The page header's actions sit above the player, so they read as actions on the video even though they act on the channel or playlist, while the video's own actions are scattered: "Mark watched" on a row of its own, "Open on YouTube" hidden in "More", and the selected list row has no "⋮" at all. There is also no way to open the channel or playlist itself on YouTube.

## What Changes

- The channel and playlist page header offers "Open channel on YouTube" / "Open playlist on YouTube", opening the source's YouTube page in a new tab (an icon button on desktop, an item in the "⋮" menu on mobile).
- The video detail pane's title row ends with the video's own actions: a "Mark watched" button (labelled from the desktop breakpoint, a bare tick below it; hidden once the video is watched or while it is not downloaded), the playback speed control, and an "Open video on YouTube" icon link opening the video in a new tab.
- **Removed:** the watched control's own row, its non-interactive "Watched" status and its "rewatch past 10%" hint; the "Open on YouTube" link inside the collapsible details section.
- The meta line starts with "Watched" when the video is watched, and hides "Synced" below the desktop breakpoint.
- Every row of the detail view's video list shows the "⋮" actions menu, the selected video's row included.

## Capabilities

### New Capabilities

### Modified Capabilities
- `web-ui`: the Detail View Page Header gains the source's YouTube link; the Video Actions Menu now appears on the selected row too; the Video Detail Watched Control becomes a title-row button; the Video Detail Title Prominence, Video Detail Links To YouTube, Collapsible Video Details and Video Detail Meta Line requirements change to match.
- `e2e-tests`: the Mobile Layout Coverage's collapsed-details check no longer expects the YouTube link inside "More"; it sits in the title row.

## Impact

- Web UI only (`web/src/components/`): `DetailHeader`, `ChannelDetail`, `PlaylistDetail`, `VideoDetail`, `VideoListPane`, `VideoWatchedControl` → `MarkWatchedButton`; `web/src/lib/sourcePath.ts` gains the YouTube channel/playlist URL builders.
- e2e tests `channel.spec.js` and `mobileWatch.spec.js` follow the new labels and placement.
- No API, backend or data changes.
