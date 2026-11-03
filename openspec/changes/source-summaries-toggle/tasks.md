# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md across Rust and web: migration `0022` registered; `summaries_enabled` on `Channel`/`Playlist` (`false` in `create`) with `with_summaries_enabled`, on both views, every response DTO and `Create*`/`Update*Request`; `ChannelSettingsChange` replacing `ChannelUpdater::update_settings`' per-setting args (handler builds it, `is_empty` → `NOTHING_TO_UPDATE`); `PlaylistSettingsChange.summaries_enabled`; creators take `summaries_enabled: bool` (ignored for now); the four create/update services take `PreferencesRepository` (wired in `serve.rs`, unused); `Preferences::summaries_available()` returning `true`; `SummariesUnavailable` variants mapped to 409; repositories read the column back as `false`, don't write it yet; web types, request types, `aChannel`/`aPlaylist` defaults, `summariesAvailable` returning `false`, `useSummariesAvailable`, `SummariesField` rendering nothing, `ChannelSettingsForm.summaries_enabled`. Done when `cargo build`, `cargo test --locked` and `npm run check` pass with no new tests.

## 2. Behaviour (TDD)

Channels handler (acceptance, real SQLite)
- [x] 2.1 `it_should_create_a_channel_with_summaries_disabled_by_default` — omitted → stored and returned `false`.
- [x] 2.2 `it_should_create_a_channel_with_summaries_enabled` — AI + summaries on; `true` → stored and returned `true`.
- [x] 2.3 `it_should_fail_to_create_a_channel_with_summaries_if_ai_is_off` — 409 `Turn on AI summaries before enabling them for a channel`, nothing stored, YouTube fake not called.
- [x] 2.4 `it_should_enable_summaries_on_a_channel` — AI + summaries on; only `summaries_enabled: true` → 200, stored `true`, other fields unchanged.
- [x] 2.5 `it_should_fail_to_enable_summaries_on_a_channel_if_summaries_are_off` — AI on, summaries off, with a new quality → 409, channel unchanged.
- [x] 2.6 `it_should_disable_summaries_on_a_channel_while_ai_is_off` — stored `false`.
- [x] 2.7 `it_should_list_channels_with_their_summaries_setting`.

Playlists handler (acceptance, real SQLite)
- [x] 2.8 `it_should_create_a_playlist_with_summaries_disabled_by_default`.
- [x] 2.9 `it_should_create_a_playlist_with_summaries_enabled` — AI + summaries on.
- [x] 2.10 `it_should_fail_to_create_a_playlist_with_summaries_if_ai_is_off` — 409 `…for a playlist`, nothing stored, YouTube fake not called.
- [x] 2.11 `it_should_enable_summaries_on_a_playlist` — AI + summaries on; only `summaries_enabled: true` → 200, stored `true`, others unchanged.
- [x] 2.12 `it_should_fail_to_enable_summaries_on_a_playlist_if_summaries_are_off` — with a new interval → 409, playlist unchanged.
- [x] 2.13 `it_should_disable_summaries_on_a_playlist_while_ai_is_off`.
- [x] 2.14 `it_should_list_playlists_with_their_summaries_setting`.

Preferences (domain unit)
- [x] 2.15 `it_should_have_summaries_available_only_with_ai_and_summaries_on` — all four combinations.

Web (Vitest)
- [x] 2.16 `preferences.test.ts` — `summariesAvailable` true only with `ai_enabled` and `summaries_enabled`; false for `undefined`.
- [x] 2.17 `channelSettings.test.ts` — `channelSettingsChanges` includes `summaries_enabled` only when changed.
- [x] 2.18 `AddChannelDialog.test.tsx` — AI summaries on: "Summarize videos" (hint "Generate an AI summary of each new video.") visible without expanding "Advanced options"; checking sends `summaries_enabled: true`; untouched sends `false`.
- [x] 2.19 `AddPlaylistDialog.test.tsx` — same as 2.18.
- [x] 2.20 `AddChannelDialog.test.tsx` — AI off: checkbox disabled, hint "Turn on AI summaries in AI settings." links to `/ai`; following it closes the dialog and routes to `/ai`.
- [x] 2.21 `EditChannelDialog.test.tsx` — prefilled checked; AI on: toggling sends `summaries_enabled` alone, no sync.
- [x] 2.22 `EditChannelDialog.test.tsx` — AI on, summaries off: disabled and shown checked for a source with it on; changing only the interval sends only the interval.
- [x] 2.23 `EditPlaylistDialog.test.tsx` — AI on: checking sends `{ summaries_enabled: true }` alone; AI off: disabled.
- [x] 2.24 `EditChannelDialog.test.tsx` — AI off, source with summaries on: checked and enabled; unchecking and saving sends `{ summaries_enabled: false }` alone (replaces 2.22's "disabled" assertion with "enabled"); `SummariesField` gains `enabledOnSource`.
- [x] 2.25 `EditPlaylistDialog.test.tsx` — same as 2.24 for playlists.

## 3. Infrastructure adapters (TDD)

- [x] 3.1 `sqlite_channel_repository`: `it_should_round_trip_the_channels_summaries_setting` — insert `true` → `true`; update to `false` → `false`.
- [x] 3.2 `sqlite_playlist_repository`: `it_should_round_trip_the_playlists_summaries_setting`.
- [x] 3.3 `sqlite_migrations`: `it_should_disable_summaries_for_sources_stored_before_the_setting`.

## 4. Verification

- [x] 4.1 `scripts/run-tests.sh --only web --only rust` passes.
- [ ] 4.2 Manual (agent): `scripts/run-local.sh --port <free port> --fresh-videos`; with Playwright MCP, AI off: open add channel (no Advanced options), assert "Summarize videos" disabled with the AI settings link, follow it, assert `/ai` and dialog closed; `curl` a playlist create with `summaries_enabled: true` and assert 409. Enable AI + summaries via `PATCH /api/preferences` (stub Ollama URL), add a playlist with the box checked, assert the `POST` body has `summaries_enabled: true`; open its Edit settings, assert checked, uncheck, save, assert `PATCH` body `{ summaries_enabled: false }`; screenshot. Stop the server, delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.3 Manual (human): visual check of the checkbox and disabled hint in the four dialogs (light/dark, mobile width).

## Workflow follow-up

- Next change: generate summaries for new videos of sources with `summaries_enabled`.
