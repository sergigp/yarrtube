# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures) across backend and web, wired end to end, with stub bodies only:
  - Migration 0012, registered.
  - `Playlist.hide_unwatched_count` (always `false`), plus `with_hide_unwatched_count`.
  - `PlaylistSettingsChange`; `PlaylistUpdater::update_settings` takes it, and the existing behaviour is kept.
  - `PlaylistView` + `PlaylistViewSearcher` replace `PlaylistSearcher`, with `unwatched_count: 0`.
  - `PlaylistVideoRepository::list` returns an empty `Vec`.
  - `VideoWatchStateUpdater::new` takes the playlist repositories, and `mark_playlist_watched` returns `Ok(())`.
  - `UpdateWatchStateError::PlaylistNotFound`, mapped to 400.
  - DTO fields and `PlaylistListItemResponse`; `mark_playlist_watched` handler; `POST /playlists/{id}/watched` route; `serve.rs` wiring.
  - Web: type fields, the `aPlaylist` defaults, `markPlaylistWatched`, `useSetPlaylistUnwatchedCountHidden`, `shownUnwatchedCount` / `orderByUnwatched` (`orderChannels` is renamed, keeping its behaviour), `unwatchedCountSettingAction`, and the new `EntryActionsMenu` / `DetailHeader` props, not rendered yet.
  - Existing test call sites are updated for the new constructors and signatures.

  Done when: `cargo build`, `cargo test --locked` and `npm run check` all pass, with no new behaviour and no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_count_unwatched_downloaded_videos_when_listing_playlists`: the list counts only downloaded, unwatched videos per playlist, and 0 for a playlist without any.
- [x] 2.2 `it_should_list_playlists_with_their_hide_unwatched_count`: a hidden playlist is listed with `hide_unwatched_count: true` and its real count.
- [x] 2.3 `it_should_create_a_playlist_showing_its_unwatched_count`: created playlists store and return `hide_unwatched_count: false`.
- [x] 2.4 `it_should_hide_a_playlists_unwatched_count`: PATCH `{hide_unwatched_count: true}` stores and returns `true`, and leaves other settings unchanged.
- [x] 2.5 `it_should_show_a_playlists_unwatched_count_again`: PATCH `{hide_unwatched_count: false}` on a hidden playlist stores `false`.
- [x] 2.6 `it_should_fail_to_update_if_nothing_to_change`: an empty PATCH returns 400 with the new `NOTHING_TO_UPDATE` message.
- [x] 2.7 `it_should_mark_every_downloaded_playlist_video_watched`: 204; downloaded videos and their channel copies are watched, and pending videos are unchanged.
- [x] 2.8 `it_should_fail_to_mark_watched_a_missing_playlist`: 400, nothing changed.
- [x] 2.9 `it_should_fail_to_mark_watched_if_invalid_playlist_id_provided`: 400.
- [x] 2.10 Vitest `sidebarSections.test.ts`, `orders entries by shown unwatched count`: hidden counts sort as 0, and the order is otherwise stable.
- [x] 2.11 Vitest `sidebarSections.test.ts`, `shows no unwatched count for an entry that hides it`.
- [x] 2.12 Vitest `Sidebar.test.tsx`, `shows a playlist's unwatched badge unless hidden`.
- [x] 2.13 Vitest `Sidebar.test.tsx`, `orders playlists by unwatched count`.
- [x] 2.14 Vitest `Sidebar.test.tsx`, `marks a playlist watched from its row menu`: `POST /api/playlists/PL1/watched`, then the badge is gone after the refetch.
- [x] 2.15 Vitest `Sidebar.test.tsx`, `hides and shows a playlist's unwatched count from its row menu`: the PATCH body is `{hide_unwatched_count: true}`, and the menu label flips.
- [x] 2.16 Vitest `Sidebar.test.tsx`, `alerts when hiding a playlist's count fails`: the alert text comes from `unwatchedCountSettingAction`, and the badge stays.
- [x] 2.17 Vitest `Sidebar.test.tsx`, `offers playlist row actions in order`: sync, "Mark all watched", home, hide count, "Edit settings", delete. This replaces the "no mark-watched" test.
- [x] 2.18 Vitest `PlaylistDetail.test.tsx`, `summarizes the playlist's unwatched count`: "3 videos · 2 unwatched", also when the count is hidden.
- [x] 2.19 Vitest `PlaylistDetail.test.tsx`, `marks the playlist watched from its page header`.
- [x] 2.20 Vitest `PlaylistDetail.test.tsx`, `hides the unwatched count from its page header`.
- [x] 2.21 Vitest `useWatchProgress.test.tsx`, `refetches the playlist list when the watched state changes`; the existing "does not refetch" test also covers the playlist list.
- [x] 2.22 Vitest: channel row and channel page header menus offer no "Hide unwatched count" / "Show unwatched count".
- [x] 2.23 `it_should_not_count_videos_of_a_playlist_hiding_its_count_in_a_category` (categories/mod.rs): channel member with 1 unwatched video, hidden playlist member with 2 others; the category's count and `unwatched_total` are 1.
- [x] 2.24 `it_should_count_once_a_video_of_a_hidden_playlist_also_in_a_counted_member`: a video in both a hidden playlist member and a channel member counts 1.
- [x] 2.25 `it_should_not_total_videos_reached_only_through_hidden_playlists`: the only unwatched video belongs to a hidden playlist member; the category's count and `unwatched_total` are 0.
- [x] 2.26 Refactor: `playlist_videos` and `channel_videos` in `VideoWatchStateUpdater` load videos with one `video_repository.find_many` instead of a `find` per entry. Behaviour-preserving; existing tests stay green, no new tests.

## 3. Infrastructure adapters (TDD)

SQLite playlist repository:

- [x] 3.1 `it_should_round_trip_the_playlists_hide_unwatched_count`: insert and update persist the column.

SQLite playlist video repository:

- [x] 3.2 `it_should_list_videos_of_every_playlist`: `list()` returns rows from every playlist.

Migrations:

- [x] 3.3 `it_should_default_existing_playlists_to_show_their_unwatched_count_when_migrating`: a pre-0012 playlist reads `hide_unwatched_count = 0`.

## 4. Verification

- [x] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass; `npm run check` passes in `web/`.
- [x] 4.2 Smoke: extend `smoke-tests/tests/playlist.spec.js`:
  - After the download, the playlist row shows badge "1".
  - "Hide unwatched count" removes the badge, and "Show unwatched count" brings it back.
  - After "mark it watched while playing", choose "Mark all watched" from the row menu: it covers `POST /playlists/{id}/watched` and leaves no badge.

  Then `scripts/run-smoke-tests.sh` passes, including the route coverage check.
- [ ] 4.3 Manual (agent):
  1. Start `scripts/run-local.sh --port <free port> --fresh-videos`.
  2. Through the HTTP API, add a playlist and a channel that share a video, and wait for the downloads.
  3. With the Playwright MCP browser, check:
     - The playlist row shows its badge, and playlists are ordered unread-first.
     - Hiding the count from the page header removes the badge and moves the row into the caught-up group.
     - "Mark all watched" on the playlist also drops the channel's badge.
     - Playing a playlist video past 90% lowers the playlist badge without a reload.
  4. Capture DOM and network evidence, plus a screenshot.
  5. Stop the server, and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.4 Manual (human): visually check the badge and the menu item icons and spacing on desktop and on a phone-width viewport, next to the "Exclude from home" item.

## Workflow follow-up

- Archive the change after review.
