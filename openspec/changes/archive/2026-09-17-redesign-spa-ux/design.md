## Context

See `proposal.md` for motivation. Relevant current state:

- `web/src/App.jsx` holds all navigation as local `useState` (`tab`), with `PlaylistsTab`/`ChannelsTab` wrapper components threading a hand-rolled `deepLink` object down to `PlaylistDetail`/`ChannelDetail` for the "jump straight to a video" case. No router dependency exists in `web/package.json`.
- `CreatePlaylistRequest`/`CreateChannelRequest` already accept `path` as a plain string, validated by `PlaylistPath::new` (no absolute paths, no `..`, no unsafe chars) and checked for uniqueness (`PlaylistCreator`) or not checked at all (`ChannelService`, today). This change does not touch any of that — the path field stays exactly as it is on the backend; only its frontend presentation changes.
- The forms (`CreatePlaylistForm.jsx`, `CreateChannelForm.jsx`) currently show `path` as a required, blank, always-visible text input the user must fill in by hand.

## Goals / Non-Goals

**Goals:**
- Replace ad-hoc tab/state navigation with real routes, without introducing a backend rendering concern (the daemon still serves one static `index.html` — see `web-ui`'s existing "Self-Contained Deployment" requirement).
- Move the path field out of the primary form flow (into "Advanced options") and remove the burden of typing it by hand, by pre-filling it client-side from the name/handle the user already entered — while keeping it visible and editable, since the backend still requires and enforces it exactly as before.
- Land the visual/interaction changes (home grid + sidebar, merged add dialog, fixed-video detail layout, visible actions, YouTube link) as one coherent pass.

**Non-Goals:**
- No backend change of any kind: `POST /api/playlists`/`POST /api/channels` request/response shapes, validation, and collision behavior (`PathAlreadyInUse` → 400) are all unchanged.
- No change to how the binary serves the SPA shell/assets (still a single embedded build, no server-side routing/rendering).
- No attempt to fetch the YouTube-resolved channel title client-side before submission — the channel path auto-fill uses the raw handle/URL the user typed, not a "nicer" display name (that would require a live API call on every keystroke; out of scope).

## Decisions

### Routing: `react-router-dom`, client-side only, `BrowserRouter`
Adds one new frontend dependency. `BrowserRouter` (not hash routing) since the daemon already serves `index.html` at `/` for any unmatched path today (confirmed by the "Serve The Application Shell" requirement) — direct-loading `/playlists/:id` needs the backend's static-file fallback to keep serving `index.html` for non-file paths, which needs verifying against the axum router's fallback during implementation (tracked in tasks.md), not assumed. The hand-rolled `deepLink` prop-drilling is replaced by a `?video=<id>` query param via `useSearchParams`.

### Path field: client-side slug auto-fill, sync-until-edited
Both forms' Path field, now inside "Advanced options":
- **Playlist mode**: pre-filled as `playlists/<slugify(name)>`, recomputed on every keystroke in the Name field.
- **Channel mode**: pre-filled as `channels/<slugify(handle)>`, where `handle` is the raw value of the Channel Handle or URL field with a leading `@` stripped (best-effort: if the value looks like a URL, take its last non-empty path segment first, mirroring what the backend's `ChannelHandle::from_url_or_handle` does, but client-side and without contacting YouTube) — recomputed on every keystroke in that field.
- `slugify`: lowercase, non-alphanumeric runs collapsed to a single `-`, leading/trailing `-` trimmed; a JS implementation mirroring the one originally scoped for the backend in the prior version of this design, now living in `web/src` instead.
- **Sync-until-edited**: the Path field keeps recomputing from its source field only until the user types into the Path field directly; once they do, auto-fill stops for that form session (reset when the dialog closes/reopens or the mode switches). This is the standard "slug field" pattern — it avoids clobbering a deliberate edit while still saving the common case of never having to touch the field.

Alternative considered (from the prior version of this design): derive and enforce the path entirely server-side with automatic collision-suffixing, dropping `path` from the API. Superseded by explicit user direction to keep the API contract unchanged; this version reverts all backend work from that plan.

### Path collisions surface through the existing error path, dialog auto-expands to show them
The backend already rejects a colliding path with a 400 (`PathAlreadyInUse` for playlists; today unchecked for channels — see Risks). Since Path now lives inside a collapsed "Advanced options" section, a user could hit that rejection without the field being visible. The add dialog SHALL expand "Advanced options" automatically when a create request fails, so the Path field (and the error message) are visible without the user having to know to look there.

### Detail view "fixed video, scrolling list" layout
Bound the detail route's content to the viewport height (`100vh` minus the header's height) with `overflow: hidden` on the two-column container; the video-list column gets `overflow-y: auto`. Pure CSS, scoped to the playlist/channel detail routes only.

### Actions visibility
Replace the Radix `DropdownMenu` ("⋯") in `PlaylistActionsMenu`/`ChannelActionsMenu` with plain visible buttons (Reconcile, Delete) in the detail header toolbar. The delete confirmation flow (`ConfirmDialog`) is unchanged.

### Add dialog tooltip
Adds `@radix-ui/react-tooltip`, consistent with the existing Radix usage (`react-dialog`, `react-dropdown-menu`) rather than a bespoke `title`-attribute tooltip, for accessible hover/focus behavior.

## Risks / Trade-offs

- [Direct-loading a detail URL depends on the backend falling back to `index.html` for unmatched paths] → Verify/adjust the axum router's static-file-serving fallback as part of implementation; this is existing `web-ui` behavior ("Serve The Application Shell") being extended to more paths, not new behavior, but needs confirming it isn't scoped to exactly `/`.
- [Channel path auto-fill uses the raw handle/URL, not the YouTube-resolved display title] → Accepted trade-off from reverting the backend-derivation plan; e.g. a channel path defaults to `channels/mkbhd` rather than `channels/marques-brownlee`. The field stays editable, so a user who wants the nicer name can type it.
- [`ChannelService` still performs no path-collision check today, unlike `PlaylistCreator`] → Pre-existing gap, unchanged by this design (it was only being closed by the now-reverted backend plan). A colliding channel path can still be silently accepted; out of scope for this change since we're explicitly not touching backend behavior.
- [Sync-until-edited means a user who backspaces the whole Path field back to empty, then keeps editing the Name field, won't get auto-fill back] → Acceptable; matches the well-known slug-field convention users are likely already familiar with, and the field is never hidden from them once Advanced options is open.

## Migration Plan

No backend deploy concerns — this version of the change ships as a frontend-only release. No data migration.
