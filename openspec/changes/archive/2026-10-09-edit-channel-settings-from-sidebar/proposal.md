## Why

Review feedback on PR #90 (`edit-channel-settings`): editing a channel's settings is only reachable from its page header, while every other per-channel action (sync, mark watched, delete) is also in the sidebar row menu. The dialog's generic "Edit channel" title doesn't say which channel is being edited, and nothing tells the user that a new quality doesn't re-download videos already on disk.

## What Changes

- Each channel row's "⋮" menu in the sidebar offers "Edit settings" (between "Mark all watched" and "Delete"), opening the same edit channel dialog. Saving from the sidebar behaves exactly as from the page header: only changed settings are sent, a changed limit triggers a sync, a failed sync is reported. Playlist rows don't offer it.
- The edit channel dialog is titled "Edit <channel name> settings" instead of "Edit channel"; the channel name is no longer shown as a separate visible line.
- While the selected quality differs from the channel's current one, the dialog shows an informational note: "The new quality applies to new videos only. Videos already downloaded keep their current quality."
- No backend or API changes.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `web-ui`: "Sidebar Row Actions" gains "Edit settings" for channel rows; "Edit Channel Dialog" changes its title; new requirement for the quality note.

## Impact

- **Web**: `Sidebar` (row menu item + owns an edit dialog), `EditChannelDialog` (title, quality note), `ChannelDetail` (uses the shared save hook), new `hooks/useSaveChannelSettings.ts` (save + sync-after-limit-change, shared by sidebar and page header), `EntryActionsMenu` doc comment.
- **Tests**: `Sidebar.test.tsx`, `EditChannelDialog.test.tsx`, `ChannelDetail.test.tsx`, `smoke-tests/tests/channel.spec.js` (dialog name).
- Already implemented in the working tree on top of PR #90, uncommitted.
