# Proposal

## Why

In a playlist or channel detail view on desktop, "Mark all watched" is an always-visible icon button in the page header, while marking the video being played is hidden in the detail pane's "⋮" menu. The bulk action is the easier one to reach, so users mark a whole playlist or channel watched by mistake when they meant the current video. The safer, more frequent action should be the visible one.

## What Changes

- Detail view page header (channel, playlist and category views), at or above the desktop breakpoint: "Mark all watched" is no longer an icon button; it moves into the "More actions" ("⋮") menu, above delete. Its confirmation ("Confirm Bulk Mark Watched") is unchanged. Below the desktop breakpoint nothing changes, as every action is already in that menu.
- Video detail pane: a labelled watched control, always visible, in its own row below the meta line:
  - "Mark watched" button while the video is downloaded and unwatched; marks it without confirmation.
  - Muted, non-interactive "Watched" status once watched, with a hint that rewatching past 10% marks it unwatched again (existing behavior).
  - Not shown while the video has not finished downloading.
- Video detail pane: its "⋮" actions menu is removed, since "Mark as watched" was its only item there. Home cards and list rows keep their "⋮" menu unchanged.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `web-ui`: "Video Actions Menu" no longer covers the video detail pane; new "Video Detail Watched Control" requirement; "Detail View Page Header" moves mark-watched into the "More actions" menu on desktop (the category view inherits this layout); "Playback Speed Control" no longer sits next to a "⋮" menu; "Video Detail Title Prominence" allows the speed control on the title row, as the app already does.

## Impact

- Web UI only: `web/src/components/DetailHeader.tsx`, `VideoDetail.tsx`, a new watched control component, and their tests; e2e tests touching the header mark-watched action or the detail pane menu.
- No backend, API or DTO changes. Rewatch-resets-watched already exists in `src/domain/video/video.rs`.
