## Why

The add playlist dialog asks for a name that YouTube already knows, and derives the folder from it, while its location controls and destination box take up most of the dialog even though most users keep the default. The channel dialog was just reshaped around a single input plus a notice; the playlist dialog should match it. Unlike a channel handle, a playlist ID is unreadable, so a sensible default name and folder need the playlist's YouTube title before submission.

## What Changes

- **BREAKING (API)**: `POST /api/playlists` no longer takes `name`. The server stores the playlist's YouTube title as its name, the same way channels store their YouTube title. If YouTube returns an empty title, the playlist ID is used.
- The playlist name is no longer restricted to filesystem-safe characters. It no longer names a folder (the storage path does), and real titles such as "AC/DC - Greatest Hits" or "Rust: from zero to hero" must be accepted. It must still not be blank.
- New `GET /api/playlists/preview?playlist=<ID or URL>` returns the playlist's ID, YouTube title and video count without persisting anything. It rejects input that isn't a playlist ID or URL and reports a playlist that doesn't exist or isn't accessible.
- The **add playlist dialog** is redesigned to match the add channel dialog, in order:
  - a playlist ID or URL field, with no name field
  - a notice, once the playlist has been looked up, stating how many videos from which playlist will be downloaded and to which absolute path, with a "change" action that expands "Advanced options"
  - collapsed "Advanced options" holding the storage location (parent folder browser and folder name) and video quality
  - the submit button
- The folder name defaults to a slug of the YouTube title, is editable under "Advanced options", and stops following the title once edited.
- The notice becomes an error, and submission stays blocked, when:
  - the input is not a playlist ID or URL, or the playlist is not found on YouTube
  - the playlist is already tracked ("Already added as <name>")
  - the destination is already used by another playlist or channel
- The destination preview box, with its "will create" / "already exists" hints, is removed from the playlist dialog. No dialog uses it any more.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `playlist-crud`: Create Playlist takes no name and stores the YouTube title. The name no longer has a filesystem-safe restriction. A new requirement adds the playlist preview endpoint.
- `web-ui`:
  - The add playlist dialog loses its name field and gains a destination notice fed by the preview.
  - Its storage location moves into "Advanced options": the storage location requirement is replaced by one covering both dialogs.
  - The destination preview requirement is removed.

## Impact

- **Backend**:
  - `YoutubePlaylistRepository` resolves a playlist's title and video count instead of only checking that it exists.
  - `PlaylistCreator` takes the name from that lookup, and a new `PlaylistPreviewer` domain service backs the preview endpoint.
  - `PlaylistName` validation is relaxed.
  - The playlists HTTP module gains the preview handler, and its create request DTO drops `name`.
- **Frontend**:
  - `web/src/components/AddPlaylistDialog.jsx` is rewritten.
  - `web/src/api.js` and `web/src/queries.js` gain the preview call.
  - `web/src/components/LocationField.jsx` loses the destination box.
  - The notice markup is shared with `AddChannelDialog.jsx`.
- **Smoke tests**:
  - `playlist.spec.js` expects the sidebar entry under the playlist's YouTube title.
  - The playlist cases in `addDialogLocation.spec.js` can no longer rely on a typed name.
  - New notice cases are added.
- **Data**: no migration. Existing playlists keep their stored names.
