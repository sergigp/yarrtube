## Why

The add dialog's storage path is a free-text field, so the parent directory has to be typed from memory against a videos share whose real layout the user cannot see. A typo in a parent segment (`playlist/music` instead of `playlists/music`) passes every validation and silently creates a stray directory tree, and the mistake is only discovered later, once videos have downloaded into the wrong place.

## What Changes

- Add an HTTP endpoint that lists the immediate subdirectories of a directory under the configured videos root, one level per request, confined to that root.
- Replace the add dialog's free-text path field with a parent-folder picker plus a separate folder-name field, in both playlist and channel mode.
- The parent folder is chosen only through the browser, never typed. Every directory in a destination is therefore one the user browsed into, and which already exists, or named explicitly in a create-folder step with that location's existing subdirectories on screen. The folder name is a single segment and rejects `/`. A directory is never brought into existence by unreviewed free-text entry.
- The browser can adopt a parent that does not exist yet, staged in the dialog and created with the rest of the destination at download time, so a new location never has to be created on the storage by hand first.
- Show the resolved absolute destination before submission, naming every directory in it that will be created — a staged parent means more than one — or reporting that it exists already, or that another playlist or channel occupies it.
- Detect a path already in use client-side, before submission, using the playlists and channels the dialog already has access to. **BREAKING** (spec-level): this retires `Advanced Options Auto-Expand On Path Conflict`, which existed only to surface that failure after a rejected submit.
- Move the location controls out of "Advanced options" into the dialog's main body. Advanced options keeps video quality and, in channel mode, the video limit.
- Create the default parent directories (`playlists/`, `channels/`) at daemon startup, not in the container image, so the picker is never empty on a fresh install.

## Capabilities

### New Capabilities
- `directory-browsing`: listing the directories under the configured videos root over HTTP, one level at a time, confined to that root so no path outside it can be enumerated.

### Modified Capabilities
- `web-ui`: `Add Dialog Advanced Options` no longer holds the storage path, and the path's free-text entry and auto-fill behavior are replaced by a browsed parent plus a named leaf with a destination preview; `Advanced Options Auto-Expand On Path Conflict` is removed, superseded by pre-submission conflict detection.
- `daemon`: adds a startup step that ensures the default parent directories exist under the videos root.

## Impact

- **New endpoint**: `GET /api/directories` — unauthenticated, like every other route on this server, so root confinement must survive symlinks that point outside the videos root, not just `..` segments.
- **New port and adapter**: a directory-listing trait and its filesystem implementation, alongside the existing `VideoFileRepository`. Listing is one level deep by design: videos are stored one directory per video, so a recursive walk of the share would be both slow and almost entirely noise.
- **Rust**: `src/application/http/mod.rs` (route), a new handler module, `src/serve.rs` (startup directory seeding, wiring).
- **Web**: `web/src/components/AddDialog.jsx` (restructured location controls), `web/src/api.js` (new client call, plus fetching playlists and channels to annotate taken directories).
- **Not the Dockerfile**: seeding the default directories in the image does not work. A bind-mounted host share replaces `/videos` at container start and masks anything the image created there.
- **Smoke tests**: `smoke-tests/helpers/addDialog.js` and the playlist and channel specs drive the path field directly and will need updating.
- **Unchanged**: `PlaylistPath` keeps its current validation and stays the single storage-path value object for both playlists and channels; the create endpoints' contracts are untouched. Existing playlists' paths are not migrated, and there is no way to change a playlist's location after creation — as before.
- **Explicitly out of scope**: fetching a playlist's title from YouTube when its URL is pasted. It is worth doing, but it addresses naming the leaf and validating the URL, not choosing the parent, which is the failure this change exists to fix.
