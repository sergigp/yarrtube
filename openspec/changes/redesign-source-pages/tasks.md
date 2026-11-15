# Tasks

## 1. Walking skeleton

- [x] 1.1 Add every type and signature from design.md across layers: migration 0033, `last_synced_at` on `Channel`/`Playlist` + views + list DTOs, `record_last_synced_at` on both repositories, `episode` on `VideoResponse`/`CategoryVideoResponse`; web `api/types` + fixture builders, `lib/sourceVideos`, `lib/playFrom`, `useRememberedChoice`, `PlayLink`, `VideoListToolbar`/`ChoiceMenu`, `WatchView`, `ChapterList`, the new `DetailHeader`/`VideoDetail`/`VideoListPane`/`CategoryPlayer`/`VideoGrid` props; remove `EditableTitle`

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_record_when_the_playlist_was_synced` — a playlist reconcile pass records its last sync time
- [x] 2.2 `it_should_keep_the_last_sync_if_listing_fails` (playlist) — a failed listing keeps it
- [x] 2.3 `it_should_record_when_the_channel_was_synced` — a channel reconcile pass records its last sync time
- [x] 2.4 `it_should_keep_the_last_sync_if_listing_fails` (channel) — a failed listing keeps it
- [x] 2.5 `it_should_list_playlists_with_their_last_sync` — `GET /api/playlists` returns it
- [x] 2.6 `it_should_list_channels_with_their_last_sync` — `GET /api/channels` returns it
- [x] 2.7 `it_should_include_the_episode_number_when_listing_playlist_videos`
- [x] 2.8 `it_should_include_the_episode_number_when_listing_channel_videos`
- [x] 2.9 `lib/sourceVideos` and `useRememberedChoice` unit tests (play next, counts, sort, sources, storage fallback)
- [x] 2.10 Playlist browse page: header meta, chips, "Ep n" cards, Play next target, filter + sort remembered, category chips, card → watching, name → back
- [x] 2.11 Watching layout: queue "Up next" position and episode numbers, Hide watched remembered, desktop chapter list beside the summary, video info source link and episode
- [x] 2.12 Channel page: meta and Play next newest unwatched, hidden once playing
- [x] 2.13 Category page: colour dot, "from N …" meta, Play newest unwatched, From source filter, sort remembered, Edit category
- [x] 2.14 Header layout (Sync + YouTube icons, rest in ⋮; stacked on phones) and always-confirmed Mark all watched, sidebar included
- [x] 2.15 Only downloaded videos count as unwatched (audit): Play next never targets a video not downloaded yet; "Unwatched" and "In progress" chips count downloaded videos only, matching the header's count

## 3. Infrastructure adapters (TDD)

- [x] 3.1 SQLite channel repository: `it_should_record_only_the_last_sync`
- [x] 3.2 SQLite channel repository: `it_should_keep_the_last_sync_when_updating_from_a_stale_read`
- [x] 3.3 SQLite playlist repository: `it_should_record_only_the_last_sync`
- [x] 3.4 SQLite playlist repository: `it_should_keep_the_last_sync_when_updating_from_a_stale_read`
- [x] 3.5 Migration 0033: rows existing before it read no last sync time

## 4. Verification

- [x] 4.1 `scripts/run-tests.sh --lint --web --rust`
- [x] 4.2 Update the e2e specs and helpers for the browse page and the ⋮ header (`enterWatching`, `chooseHeaderAction`, downloaded detection, category rename/edit, page header coverage), then `scripts/run-tests.sh --e2e`
- [ ] 4.3 Manual (agent): `scripts/run-local.sh --port <free port> --fresh-videos`, seed a playlist, a channel and a category via the HTTP API, and with the Playwright MCP browser at 1440x900 and 390x844 check: playlist browse (meta with "synced …", "Ep n" cards, chips, sort), Play next resumes, watching layout (queue header, Hide watched, chapters beside the summary), channel, category (From filter); then stop the server and delete `yarrtube.sqlite3` and the fresh videos dir. (Done locally at both sizes: "synced 27s ago", "Ep n" cards, chips, Play next skipping a watched Ep 1 and resuming Ep 2 at 600 s, queue "3 of 6 · playlist order" with episode numbers, Hide watched kept after reload, chapter list, From filter kept after reload, stacked mobile header. Not seen locally: chapters *beside* an AI summary, as AI summaries are off on a fresh server; seen only against the remote library.)
- [ ] 4.4 Manual (human): visual judgement of the pages against the Claude Design mockups, light and dark themes
