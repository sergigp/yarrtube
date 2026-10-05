## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures), wired end-to-end with trivial bodies:
  - Backend:
    - migration 0007 file + registration (real SQL, so the column exists)
    - `Playlist.exclude_from_home`, `Playlist::create(.., exclude_from_home, ..)`, `with_exclude_from_home` returning `self` unchanged
    - `PlaylistRepository::update` on `SqlitePlaylistRepository` returning `Ok(())`; row mapping reads `false` for now
    - `PlaylistCreatorApi::create(.., exclude_from_home)`, ignored for now
    - `UpdatePlaylistError`; `PlaylistUpdater` + `PlaylistUpdaterApi::update_exclude_from_home` returning `NotFound`
    - DTOs: `CreatePlaylistRequest.exclude_from_home`, `UpdatePlaylistRequest`, `PlaylistResponse.exclude_from_home` (from the entity)
    - `update_playlist` handler, `MISSING_EXCLUDE_FROM_HOME`, `PATCH /playlists/{id}` route, `ApiServices.playlist_updater`, built in `serve.rs`
  - Existing `Playlist::create(..)` call sites pass `false`; existing `PlaylistResponse` expectations get `exclude_from_home: false`.
  - Web:
    - `PlaylistListItem.exclude_from_home` + `aPlaylist` default `false`
    - `CreatePlaylistRequest.exclude_from_home`, `updatePlaylist` client, `useSetPlaylistExcludedFromHome`
    - `EntryActionsMenu` component rendering a ⋮ trigger with no items yet, not placed in any view
    - `VideoActionsMenu` `excludablePlaylist` prop, unused

  Done when `cargo build` succeeds and `cargo test --locked` and `npm run check` pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_create_a_playlist_shown_on_home_by_default`: no `exclude_from_home` → 201 with `false`, stored with `false`
- [x] 2.2 `it_should_create_a_playlist_excluded_from_home`: `Some(true)` → response and stored playlist have `true`
- [x] 2.3 `it_should_keep_exclude_from_home_of_an_existing_playlist_on_duplicate_create`: stored `false`, request `true` → 200 with the existing record, storage unchanged
- [x] 2.4 `it_should_list_playlists_with_their_exclude_from_home`: seeded mix → each flag in the list response
- [x] 2.5 `it_should_exclude_a_playlist_from_home`: PATCH `true` on a shown playlist → 200 with `true`, stored as `original.with_exclude_from_home(true)`
- [x] 2.6 `it_should_include_a_playlist_in_home_again`: PATCH `false` on an excluded playlist → `false` returned and stored
- [x] 2.7 `it_should_leave_a_playlist_unchanged_if_already_set`: PATCH `true` on an excluded playlist → 200, storage unchanged
- [ ] 2.8 `it_should_fail_to_update_an_unknown_playlist`: 404 `playlist <id> not found`, nothing stored
- [ ] 2.9 `it_should_fail_to_update_if_exclude_from_home_missing`: 400 `MISSING_EXCLUDE_FROM_HOME`
- [ ] 2.10 `it_should_fail_to_update_if_invalid_id_provided`: 400 with the `PlaylistId` validation message
- [ ] 2.11 `it_should_leave_videos_of_a_playlist_excluded_from_home_out_of_home`: in-progress, short and long videos of an excluded playlist → `empty_home()`
- [ ] 2.12 `it_should_show_a_video_of_an_excluded_playlist_through_another_source_on_home`: same video in an excluded and a shown playlist → once under latest, sourced from the shown playlist
- [ ] 2.13 `it_should_fill_home_with_shown_playlists_if_an_excluded_one_has_newer_videos`: only the shown playlist's videos, in sync order
- [ ] 2.14 `AddPlaylistDialog` "sends exclude_from_home false by default"
- [ ] 2.15 `AddPlaylistDialog` "sends exclude_from_home true when "Exclude from home" is checked": checkbox under Advanced options
- [ ] 2.16 `VideoActionsMenu` "excludes the playlist from home when its item is chosen": PATCH routed, home and library refetched
- [ ] 2.17 `VideoActionsMenu` "offers no exclude item without a playlist"
- [ ] 2.18 `VideoActionsMenu` "alerts and leaves the playlist as it was when excluding fails"
- [ ] 2.19 `Home` "excluding a card's playlist from home removes its cards": `Home` passes the playlist source to `VideoActionsMenu`
- [ ] 2.20 `Home` "a channel card offers no exclude item"
- [ ] 2.21 `Sidebar` "excludes a playlist from home from its row menu": label flips to "Include in home"
- [ ] 2.22 `Sidebar` "includes an excluded playlist in home from its row menu": PATCH `{exclude_from_home: false}`
- [ ] 2.23 `Sidebar` "a channel row menu offers no home item"
- [ ] 2.24 `Sidebar` "alerts when changing a playlist's home setting fails"
- [ ] 2.25 `PlaylistDetail` "keeps Sync visible and the other actions in the ⋮ menu": `DetailHeader` uses `EntryActionsMenu`
- [ ] 2.26 `PlaylistDetail` "excludes the playlist from home from its page header menu"
- [ ] 2.27 `PlaylistDetail` "deletes the playlist from its page header menu after confirming"
- [ ] 2.28 `ChannelDetail` "marks the channel watched from its page header menu"

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 `SqlitePlaylistRepository` `it_should_return_the_exclude_from_home_of_an_inserted_playlist`: insert with `true` → `find` returns `true`
- [ ] 3.2 `SqlitePlaylistRepository` `it_should_update_an_existing_playlist`: `update` with `with_exclude_from_home(true)` → `find` returns it, other fields unchanged
- [ ] 3.3 `sqlite_migrations` `it_should_default_existing_playlists_to_shown_on_home_when_migrating`: a pre-0007 playlist row has `exclude_from_home = 0` after migrating

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass
- [ ] 4.2 `npm run check` passes in `web/`
- [ ] 4.3 Extend `smoke-tests/tests/playlist.spec.js`: exclude the playlist from home through its ⋮ menu → its card leaves home; include it again → it returns. Run `scripts/run-smoke-tests.sh` and it passes
- [ ] 4.4 Manual check with `scripts/run-local.sh`:
  - (a) Add a playlist with "Exclude from home" checked. Its videos never show on home.
  - (b) From a home card of a shown playlist, choose `Exclude "<name>" from home`. All its cards disappear.
  - (c) Sidebar and page header ⋮ show "Include in home" for that playlist, and choosing it brings the cards back.
  - (d) Channel page header: Sync visible, ⋮ holds mark watched and delete, and delete asks for confirmation.
