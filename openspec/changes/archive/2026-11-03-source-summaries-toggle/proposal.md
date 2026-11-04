## Why

Video summaries (Ollama) are coming, but generating one per video for every
tracked source is slow and unwanted for most of them. Each channel and playlist
needs its own opt-in before generation is built, so the generator can just read it.

## What Changes

- Channels and playlists get a stored `summaries_enabled` setting, `false` by
  default and for every source stored before this change.
- Create and update endpoints for both accept an optional `summaries_enabled`
  boolean; every channel/playlist response includes it.
- The add and edit dialogs for both channels and playlists show a "Summarize
  videos" checkbox.
- While AI summaries aren't on globally (the "Enable AI" switch or "Summaries
  enabled" on `/ai` is off), the checkbox is disabled unless the source already
  has summaries on (so they can still be switched off), and its hint links to the AI settings page. In the add dialogs it is always
  visible, not inside "Advanced options".
- The daemon rejects enabling summaries on a source (create or update) unless
  both AI and summaries are on in the preferences; disabling is always allowed.
- No summary is generated yet: the setting is stored and shown only.

## Capabilities

### New Capabilities
- `source-summaries`: the per-channel/per-playlist summaries opt-in: its default,
  setting it on creation, the global AI summaries precondition, and exposing it
  in responses.

### Modified Capabilities
- `channel-crud`: "Update Channel Settings" also accepts `summaries_enabled`,
  rejected as `true` while AI summaries are off.
- `playlist-crud`: "Update Playlist Settings" also accepts `summaries_enabled`,
  rejected as `true` while AI summaries are off.
- `web-ui`: add and edit dialogs for channels and playlists gain the "Summarize
  videos" checkbox, gated on the global AI summaries settings.

## Impact

- DB: migration `0022` adds `summaries_enabled` to `channels` and `playlists`.
- Rust: `Channel`, `Playlist`, their views, creators, updaters, SQLite
  repositories, HTTP DTOs and handlers. `ChannelUpdater` takes a
  `ChannelSettingsChange` (mirroring `PlaylistSettingsChange`) instead of one
  argument per setting. The channel/playlist creators and updaters read the
  preferences to check AI summaries are on.
- Web: API types/requests, fixture builders, the four source dialogs, a shared
  `SummariesField` and a `summariesAvailable` preference helper.
- No new routes. New 409 response on create/update when enabling summaries while AI summaries are off.
