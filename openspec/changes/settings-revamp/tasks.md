# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every type and signature from design.md's Types & Signatures, wired end-to-end with trivial bodies: `PlexPlaylistSettings` (+ `PlaylistWatched`, `PlaylistOrder`) in `Preferences`/`PreferencesChange` and the preferences DTOs; `domain/plex_status` (`PlexSyncKind`, `PlexSyncRun`, `PlexSyncOutcome`, `PlexStatus`, `PlexConnection`, `PlexSyncStatus`, `RequestPlexSyncError`); `PlexIdentity` and `PlexLibraryRepository::identity()` replacing `server_id`/`viewer_id` (callers and existing tests moved to it); `create_smart_playlist(.., options)`; `CategoryPlaylist.options`; `PlexSyncRunRepository` + `SqlitePlexSyncRunRepository`; migration `0032_plex_settings.sql`; new deps on `PlexWatchSyncer`, `PlexLabelSyncer`, `PreferencesUpdater`; `PlexStatusSearcher`, `PlexSyncRequester`; `http/plex` handlers, DTOs, `ApiServices` fields and routes; composition in `serve.rs`; fakes for `PlexSyncRunRepository` and the changed Plex port. Web: types, `usePlexStatus`/`useRequestPlexSync`, `lib/plexStatus.ts` stubs, `SettingsView` + `PlexSettingsView` placeholders, routes under `/settings` with redirects from `/preferences` and `/ai`, `aPlexStatus`/`aPlexSync` builders and the new `aPreferences` fields. Done when `cargo build` and `npm run build` succeed and all existing tests pass.

## 2. Behaviour (TDD)

Watch sync (`sync_plex_watch_state_task`)
- [x] 2.1 `it_should_record_the_plex_version_and_libraries_when_a_watch_sync_reaches_plex` — a reached run is recorded with version and libraries.
- [x] 2.2 `it_should_record_the_error_when_a_watch_sync_cant_reach_plex` — a failed run is recorded with the error.

Label sync (`sync_plex_labels_task`)
- [ ] 2.3 `it_should_record_the_plex_version_and_libraries_when_a_label_sync_reaches_plex`
- [ ] 2.4 `it_should_record_the_error_when_a_label_sync_cant_reach_plex`
- [ ] 2.5 `it_should_create_playlists_that_drop_watched_episodes_when_the_preference_says_drop` — the label syncer reads the playlist preferences.
- [ ] 2.6 `it_should_create_playlists_oldest_first_when_the_preference_says_so`
- [ ] 2.7 `it_should_keep_a_playlist_whose_options_match_the_preferences`
- [ ] 2.8 `it_should_replace_a_playlist_whose_options_differ_from_the_preferences`
- [ ] 2.9 `it_should_replace_a_playlist_whose_options_are_unknown`

Plex status endpoints (`http/plex`)
- [ ] 2.10 `it_should_report_plex_not_configured`
- [ ] 2.11 `it_should_report_each_sync_with_its_next_run_before_any_run`
- [ ] 2.12 `it_should_report_the_connection_from_the_latest_run`
- [ ] 2.13 `it_should_keep_the_version_and_libraries_last_seen_when_the_latest_run_failed`
- [ ] 2.14 `it_should_report_a_running_sync`
- [ ] 2.15 `it_should_bring_the_pending_sync_forward_when_synced_now`
- [ ] 2.16 `it_should_schedule_a_sync_now_when_none_is_pending`
- [ ] 2.17 `it_should_leave_one_pending_sync_when_synced_now_twice`
- [ ] 2.18 `it_should_respond_404_for_an_unknown_sync`
- [ ] 2.19 `it_should_respond_409_when_plex_isnt_configured`

Preferences (`http/preferences`)
- [ ] 2.20 `it_should_read_the_plex_playlist_preferences_as_their_defaults`
- [ ] 2.21 `it_should_save_a_plex_playlist_preference`
- [ ] 2.22 `it_should_reject_an_invalid_plex_playlist_order`
- [ ] 2.23 `it_should_request_a_label_sync_now_when_a_playlist_preference_changes`
- [ ] 2.24 `it_should_not_request_a_label_sync_when_a_playlist_preference_is_saved_unchanged`
- [ ] 2.25 `it_should_save_a_playlist_preference_without_plex_configured`

Web (Vitest), one red-green-refactor cycle each
- [ ] 2.26 `lib/plexStatus.test.ts` — `plexVersionLabel` drops the build suffix.
- [ ] 2.27 `lib/plexStatus.test.ts` — `isSyncing`: running; requested and not finished since; idle.
- [ ] 2.28 `SettingsMenu.test.tsx` — the menu lists Tasks, Settings, Report a bug; "Settings" goes to `/settings/general`.
- [ ] 2.29 `SettingsView.test.tsx` — the section nav switches route and marks the open section.
- [ ] 2.30 `SettingsView.test.tsx` — `/settings` opens General; `/preferences` and `/ai` redirect.
- [ ] 2.31 `PreferencesView.test.tsx` — headed "General", "Saved" next to it (adapt existing tests).
- [ ] 2.32 Source dialog tests — "AI settings" links to `/settings/ai` (adapt existing tests).
- [ ] 2.33 `PlexSettingsView.test.tsx` — not configured: the three variables, the guide link, no "Sync now".
- [ ] 2.34 `PlexSettingsView.test.tsx` — connected: "Connected", "Plex Media Server 1.43.5.11079", libraries, relative time.
- [ ] 2.35 `PlexSettingsView.test.tsx` — unreachable: "Can't reach Plex", the error, last seen version.
- [ ] 2.36 `PlexSettingsView.test.tsx` — no sync yet: status shows after the first sync.
- [ ] 2.37 `PlexSettingsView.test.tsx` — sync rows: last run, its error, next run.
- [ ] 2.38 `PlexSettingsView.test.tsx` — "Sync now" POSTs and shows "Syncing…" disabled until a newer run.
- [ ] 2.39 `PlexSettingsView.test.tsx` — "Sync now" request fails: error shown, button enabled again.
- [ ] 2.40 `PlexSettingsView.test.tsx` — playlist options: defaults selected; "Drop out" PATCHes only `plex_playlist_watched` and shows "Saved".
- [ ] 2.41 `PlexSettingsView.test.tsx` — playlist option save fails: error shown, previous choice selected.
- [ ] 2.42 `PlexSettingsView.test.tsx` — categories off: options disabled with the note.

## 3. Infrastructure adapters (TDD)

`HttpPlexLibraryRepository`
- [ ] 3.1 `it_should_read_the_server_identity_with_its_version`
- [ ] 3.2 `it_should_create_a_smart_playlist_filtered_by_label` (update: passes default options, URI unchanged)
- [ ] 3.3 `it_should_create_a_smart_playlist_of_unwatched_episodes_oldest_first`

`SqlitePlexSyncRunRepository`
- [ ] 3.4 `it_should_list_the_latest_run_of_each_sync`
- [ ] 3.5 `it_should_replace_a_syncs_previous_run`

`SqliteCategoryPlaylistRepository`
- [ ] 3.6 `it_should_round_trip_a_playlists_options`
- [ ] 3.7 `it_should_read_a_playlist_without_options_as_unknown`

`SqlitePreferencesRepository`
- [ ] 3.8 `it_should_round_trip_the_plex_playlist_preferences`
- [ ] 3.9 `it_should_read_missing_plex_playlist_keys_as_defaults`

## 4. Docs

- [ ] 4.1 `doc/CATEGORIES.md` (In Plex): the two playlist options in Settings → Plex; Plex Web and the Mac app show no watched badge in playlists.
- [ ] 4.2 `doc/PLEX.md`: a "Status and syncs" note pointing at Settings → Plex; existing playlists are replaced once after upgrading.

## 5. Verification

- [ ] 5.1 `scripts/run-tests.sh --lint --web --rust`.
- [ ] 5.2 Manual (agent): `scripts/run-local.sh --port <free port> --fresh-videos` without Plex vars; with the Playwright MCP browser: gear menu shows Tasks / Settings / Report a bug; `/settings` lands on General; nav switches sections and URL; `/preferences` and `/ai` redirect; on a phone-width viewport the nav is tabs with no horizontal scroll; Plex section shows the not-configured help; General toggles still save. Stop the server, delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 5.3 Manual (human): against the real Plex, Settings → Plex shows "Connected", the version and the yarrtube libraries; "Sync now" on each row shows "Syncing…" then a new last run; choosing "Drop out" replaces the category playlists within a minute and a watched episode leaves them; "Oldest first" reverses them; the first label sync after the upgrade replaces playlists made by older versions.
