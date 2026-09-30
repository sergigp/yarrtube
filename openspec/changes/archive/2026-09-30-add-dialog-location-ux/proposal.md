# Add Dialog Location UX

## Why

Choosing where a playlist or channel is stored is the only irreversible decision in the add dialogs (there is no update endpoint — a wrong path means delete, re-add and re-download), yet today it hides behind "Advanced options" and takes three clicks of tree navigation to reach a folder the user has already used many times (e.g. `playlists/kids`). The dialog is also too narrow for its own expanded content, which clips off the right edge.

## What Changes

- The storage location becomes a first-class, always-visible **"Save to" suggestion list** directly under the ID/URL field: the mode root (`playlists/` or `channels/`) plus the parent folders that already contain tracked items, derived client-side from the already-fetched playlist/channel paths, ordered by most recent use and capped. One click selects a destination parent.
- A **"Choose another folder…" entry** at the end of the list expands the existing tree browser inline; picking a folder there collapses the browser and shows the pick as the selected entry. The tree browser itself is unchanged.
- The **last-used parent is remembered per mode** (localStorage) and preselected on the next open, validated against the current suggestions before use; stale values fall back to the mode root. Repeat adds into the same folder need zero location clicks.
- The destination notice ("All N videos … will be downloaded to …") stays below the suggestion list as pure confirmation; its **"change" link is removed**.
- **"Advanced options" shrinks** to the folder-name override and video quality (plus video limit for channels); the parent folder control moves out of it entirely.
- Both dialogs get a **design pass**: wider dialog (`sm:max-w-lg`-class sizing), more generous spacing so the form breathes, and a fix for the horizontal clipping seen when the browser panel lists long "in use by …" labels.
- Applies identically to the add playlist and add channel dialogs.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `web-ui`: the requirements "Add Dialog Storage Location In Advanced Options", "Add Dialog Parent Folder Browsing", "Add Playlist Destination Notice", "Add Channel Destination Notice" and "Add Dialog Download Options" change — the location moves out of Advanced options into an always-visible suggestion list with a remembered default, the browser opens from the list's last entry instead of an Advanced control, and the notices lose their "change" action.

## Impact

- Frontend only (`web/`): `AddPlaylistDialog.tsx`, `AddChannelDialog.tsx`, `LocationField.tsx` (split into suggestion list + browser), `DestinationNotice.tsx`, their tests, and a small pure module in `src/lib/` for deriving/ordering suggested parents (unit-testable).
- No backend, API or DTO changes: suggestions derive from `GET /api/playlists` + `GET /api/channels` paths already fetched for the occupied-folder map; recency uses playlist `created_at` where available, frequency otherwise.
- New localStorage key per mode for the remembered parent; must degrade gracefully when storage is unavailable (existing `src/test/setup.ts` stubs cover jsdom).
