# Proposal

## Why

"Mark all watched" on a channel, playlist or category takes effect on a single click and cannot be undone in bulk: reverting it means unwatching videos one by one, and it also marks every other stored copy of those videos. A stray click wipes the user's watch progress for a whole source.

## What Changes

- Marking a channel, playlist or category watched asks for confirmation first when it would mark 2 or more unwatched videos, from every place it is offered: the channel, playlist and category view headers (icon buttons on desktop, "⋮" menu below it) and the channel and playlist sidebar row menus.
- The confirmation states how many unwatched videos will be marked and that it cannot be undone in bulk. Its confirm action is styled as destructive.
- With 0 or 1 unwatched videos, the action takes effect immediately, as today (one video is no worse than the single-video "Mark as watched", which stays unconfirmed).
- The count is the real unwatched count, also for a playlist whose sidebar count is hidden.
- The shared confirmation dialog gains a configurable pending label (it currently always says "Deleting…").

## Capabilities

### New Capabilities

### Modified Capabilities
- `web-ui`: bulk mark-watched (channel, playlist, category) requires confirmation above 1 unwatched video; the requirements that state it takes effect without confirmation change accordingly.

## Impact

- Frontend only (`web/`): `ConfirmDialog`, `DetailHeader`, `EntryActionsMenu`, and the components passing them unwatched counts (`Sidebar`, `ChannelDetail`, `PlaylistDetail`, `CategoryDetail`), plus their tests.
- No backend, API or DTO changes: every source already reports `unwatched_count`.
