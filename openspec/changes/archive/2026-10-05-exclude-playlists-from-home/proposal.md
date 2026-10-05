## Why

Some tracked playlists aren't meant for the user's own viewing (e.g. kids' TV shows with ~100 episodes), yet every downloaded video of every playlist feeds the home view, so one such playlist floods "Quick watches" and "Latest videos". The user needs a per-playlist way to keep those videos off home while still tracking and downloading them.

## What Changes

- Playlists gain an `exclude_from_home` setting, `false` by default and stored in a new `playlists` column (existing playlists become `false`).
- Creating a playlist accepts an optional `exclude_from_home`; the playlist responses expose it.
- New `PATCH /playlists/{id}` endpoint that changes a playlist's `exclude_from_home`.
- The home endpoint leaves out every video sourced from an excluded playlist, in all three sections. A video also tracked by a non-excluded source still appears through that source.
- Add playlist dialog: an "Exclude from home" checkbox.
- Home card actions menu: for playlist-sourced cards, an `Exclude "<playlist name>" from home` item.
- Sidebar playlist row menu: an "Exclude from home" / "Include in home" item, depending on the current setting.
- Detail view page header: Sync stays a visible button; mark watched (channels), exclude/include in home (playlists) and delete move into a "⋮" menu, for both playlists and channels.
- Channels are unaffected: following a channel already expresses wanting its updates on home.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `playlist-crud`: creation accepts `exclude_from_home`; listing returns it; new requirement to update a playlist's `exclude_from_home`.
- `video-listing`: home listing leaves out videos sourced from playlists excluded from home.
- `web-ui`: add playlist dialog checkbox; exclude item on home card menu; exclude/include item on sidebar playlist row menu; detail view page header collapses all actions but Sync into a "⋮" menu (affects channels' mark watched control too).

## Impact

- **DB**: new migration adding `playlists.exclude_from_home INTEGER NOT NULL DEFAULT 0`.
- **Domain**: `Playlist` gains the field; `VideoSearcher::list_home` filters excluded playlists; a new playlist update use case.
- **HTTP**: `CreatePlaylistRequest` / `PlaylistResponse` DTOs change; new `PATCH /playlists/{id}` route.
- **Web**: `api/types.ts` + `aPlaylist` builder, API client + query hook, `AddPlaylistDialog`, `VideoActionsMenu` / `Home`, `Sidebar`, `DetailHeader` (shared by `PlaylistDetail` and `ChannelDetail`).
- No breaking API change: the new request field is optional.
