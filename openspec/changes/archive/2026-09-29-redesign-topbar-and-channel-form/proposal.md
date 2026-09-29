## Why

Adding a channel means filling in a dialog whose main body is taken up by location controls: parent folder, folder name, folder browser and a destination box with creation hints. Most users accept the default `channels/<handle>`, so for them all of that is noise, while the one default that does surprise people, that only the latest 3 videos are downloaded, sits hidden in "Advanced options". The single top-right "Add" button with its Playlist/Channel tabs is also disconnected from the lists it adds to.

## What Changes

- **BREAKING (UI)**: The header's "Add" button and the unified add dialog with its Playlist/Channel tabs are removed.
- The sidebar's Channels and Playlists sections each get an add entry ("Add channel", "Add playlist") between the section heading and its list. Each opens its own dialog. On mobile, the sidebar drawer closes first.
- The new **add channel dialog** has, in order:
  - a channel handle or URL field
  - a notice, once a handle is entered, stating how many of the latest videos will be downloaded and to which absolute path, with a "change" action that expands "Advanced options"
  - collapsed "Advanced options", which now also holds the storage location (parent folder browser and folder name) next to video quality and video limit
  - the submit button

  When the destination is already used by another channel or playlist, the notice becomes an error naming that channel or playlist, and submission stays blocked.
- The add channel dialog drops the destination box and its "will create a new folder" / "already exists" hints.
- The **add playlist dialog** is the current playlist form without the tabs. Redesigning it is left to a later change.
- The header's "Tasks" button moves into a settings (gear) menu at the top right. For now "Tasks" is its only item.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`:
  - The unified add dialog is replaced by separate add channel and add playlist dialogs, opened from the sidebar.
  - The storage location, destination preview and download options requirements are split between the two dialogs.
  - The channel dialog gains a destination notice.
  - The header gains a settings menu holding the link to the tasks view.

## Impact

- Frontend only, with no API or backend changes.
- Changed: `web/src/App.jsx` (header, dialog state), `web/src/components/Sidebar.jsx` (add entries) and `web/src/components/LocationField.jsx` (reports more of its state; the destination box becomes optional).
- `web/src/components/AddDialog.jsx` is replaced by `AddChannelDialog.jsx` and `AddPlaylistDialog.jsx`, plus a shared `VideoQualityField.jsx` and a new `SettingsMenu.jsx`.
- Smoke tests: `smoke-tests/helpers/addDialog.js`, `channel.spec.js`, `playlist.spec.js`, `addDialogLocation.spec.js` and `tasks.spec.js` are updated for the new entry points.
