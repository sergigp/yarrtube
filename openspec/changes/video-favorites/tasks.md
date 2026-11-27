# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every type and signature from design.md's Types & Signatures, wired end-to-end with trivial bodies (no `todo!()`):
  - Migration `migrations/0035_video_favorites.sql` (registered like 0034).
  - Domain: `Video.favorited_at` + `favorite` / `unfavorite` (return `None`) / `is_favorite` (`false`) / `with_favorited_at` (returns `self`); `FavoriteVideoError`; `VideoFavoriter` + `VideoFavoriterApi` (return `Ok(())`) exported from `domain::services`; `VideoDiscarder` taking a `VideoFavoriter` (unused); `ChannelVideo.kept` / `PlaylistVideo.kept` + `keep` (returns `self`); `VideoView.kept`, `HomeVideoView.favorite`, `ChannelView.favorite_count`, `PlaylistView.favorite_count` (all `false` / `0`); `VideoSearcherApi::list_favorites` (empty `Vec`); `VideoRepository::list_favorites` (empty `Vec`) in the SQLite repo and every fake.
  - Infrastructure: SQLite repos read and write `favorited_at` / `kept` columns (round-trip not yet tested).
  - Application: `favorite_video` / `unfavorite_video` (204) at `PUT|DELETE /videos/{id}/favorite`, `list_favorite_videos` at `GET /favorites/videos`; `ApiServices.video_favoriter` built in `serve.rs` and the HTTP test support; DTO fields `favorite`, `kept`, `favorite_count`.
  - Web: fields in `api/types.ts` and fixture builders (`aVideo`, `aHomeVideo`, `aChannel`, `aPlaylist`, category video) defaulting to `false` / `0`; `favoriteVideo` / `unfavoriteVideo` / `fetchFavoriteVideos` in `api/client.ts`; `useFavoriteVideos` / `useSetVideoFavorite` in `api/queries.ts`; `lib/sourceDelete.ts` (returns today's static texts); empty `FavoriteButton`, `UnfavoriteKeptDialog`, `FavoritesDetail` components; `/favorites` route in `App.tsx`.
  - Done when `cargo build` succeeds and `scripts/run-tests.sh --lint --web --rust` passes with no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_favorite_a_video`: 204, the video is a favorite with `favorited_at` = now (`Video::favorite`, `VideoFavoriter::favorite`).
- [x] 2.2 `it_should_favorite_every_copy_of_the_youtube_video`: channel and playlist copies both favorite, same time.
- [x] 2.3 `it_should_keep_the_favorite_time_when_favorited_again`: 204, `favorited_at` unchanged.
- [x] 2.4 `it_should_unfavorite_every_copy_of_the_youtube_video`: 204, neither copy favorite.
- [x] 2.5 `it_should_leave_a_video_that_is_not_a_favorite_unchanged_when_unfavorited`: 204, no write.
- [x] 2.6 `it_should_refuse_to_favorite_a_discarded_video`: 409, not favorite.
- [x] 2.7 `it_should_fail_to_favorite_an_unknown_video`: 404 for favorite and unfavorite.
- [x] 2.8 `it_should_unfavorite_every_copy_when_discarding_a_favorite`: `Video::discard` clears `favorited_at`, and `VideoDiscarder` calls `unfavorite`.
- [x] 2.9 `it_should_report_favorite_and_kept_in_a_channel_list`: `VideoView.kept` from the membership row, plus `VideoResponse.favorite` / `kept`.
- [x] 2.10 `it_should_report_favorite_on_home_cards`: `HomeVideoView.favorite`.
- [x] 2.11 `it_should_list_favorites_most_recently_favorited_first`: `VideoSearcher::list_favorites`.
- [x] 2.12 `it_should_list_a_favorite_tracked_twice_once_from_its_channel`.
- [x] 2.13 `it_should_omit_unavailable_favorites_from_the_favorites_list`.
- [x] 2.14 `it_should_list_no_favorites`: 200, `[]`.
- [x] 2.15 `it_should_list_a_channel_with_its_favorite_count`: `ChannelViewSearcher`.
- [x] 2.16 `it_should_list_a_playlist_with_its_favorite_count`: `PlaylistViewSearcher`.
- [ ] 2.17 `reconcile_channel_task::it_should_request_one_more_upload_per_favorite`: limit 3, two favorites (one kept), so 5 requested.
- [ ] 2.18 `reconcile_channel_task::it_should_not_count_a_favorite_toward_the_limit`: limit 3, A B C D with B favorite, so all four stored.
- [ ] 2.19 `reconcile_channel_task::it_should_keep_a_favorite_that_ages_out`: stored, kept, no `VideoRemovedFromChannel` (`keep_or_remove_videos_not_in`).
- [ ] 2.20 `reconcile_channel_task::it_should_keep_a_favorite_deleted_on_youtube`.
- [ ] 2.21 `reconcile_channel_task::it_should_keep_the_pending_download_of_a_kept_favorite`.
- [ ] 2.22 `reconcile_channel_task::it_should_place_kept_videos_after_the_listed_ones`: order A, B, C, D (`ChannelVideo::keep`).
- [ ] 2.23 `reconcile_channel_task::it_should_stop_keeping_a_kept_video_listed_again`: `refresh_known_video` clears `kept`.
- [ ] 2.24 `reconcile_channel_task::it_should_evict_a_kept_video_once_unfavorited`.
- [ ] 2.25 `reconcile_channel_task::it_should_keep_a_video_favorited_during_the_listing`: `is_favorite_now` re-read.
- [ ] 2.26 `reconcile_channel_task::it_should_store_a_new_copy_of_a_favorite_as_a_favorite`: `inherited_favorite`.
- [ ] 2.27 `reconcile_channel_task::it_should_not_delete_the_files_of_a_kept_favorite`: filesystem reconcile leaves its episode files.
- [ ] 2.28 `reconcile_playlist_task::it_should_keep_a_favorite_removed_from_the_playlist`: stays `S01E004`, kept, no `VideoRemovedFromPlaylist`.
- [ ] 2.29 `reconcile_playlist_task::it_should_not_reuse_the_episode_number_of_a_kept_favorite`: next new video is `7`.
- [ ] 2.30 `reconcile_playlist_task::it_should_place_kept_videos_after_the_listed_ones_in_a_playlist`.
- [ ] 2.31 `reconcile_playlist_task::it_should_stop_keeping_a_video_re_added_to_the_playlist`.
- [ ] 2.32 `reconcile_playlist_task::it_should_remove_a_kept_video_once_unfavorited_from_the_playlist`.
- [ ] 2.33 `reconcile_playlist_task::it_should_keep_a_video_favorited_during_the_playlist_listing`.
- [ ] 2.34 `reconcile_playlist_task::it_should_store_a_new_playlist_copy_of_a_favorite_as_a_favorite`.
- [ ] 2.35 Web `FavoriteButton` "favorites a video": `PUT /api/videos/v1/favorite`, toggle becomes "Remove from favorites"; placed in `VideoDetail`.
- [ ] 2.36 Web `FavoriteButton` "unfavorites a favorite without asking".
- [ ] 2.37 Web `FavoriteButton` "asks before unfavoriting a kept video": `UnfavoriteKeptDialog` text names the title and source; confirm sends `DELETE`.
- [ ] 2.38 Web `FavoriteButton` "sends nothing when unfavoriting a kept video is cancelled".
- [ ] 2.39 Web `FavoriteButton` "tells the user when favoriting fails".
- [ ] 2.40 Web `VideoActionsMenu` "offers Add to favorites and Remove from favorites": new `favorite` prop, passed by every card caller.
- [ ] 2.41 Web `VideoGrid` "shows a star on favorite cards and a Kept badge on kept ones": `ThumbnailOverlay`.
- [ ] 2.42 Web `Sidebar` "shows Favorites above Categories", "above Channels with categories off", "marks Favorites active on /favorites".
- [ ] 2.43 Web `FavoritesDetail` "lists favorites in server order with their source", "says No favorites yet", "drops a video once unfavorited".
- [ ] 2.44 Web `lib/sourceDelete.test.ts`: wording for 0, 1 and 2 favorites.
- [ ] 2.45 Web `Sidebar` "warns how many favorites deleting a channel deletes": sidebar rows and `ChannelDetail` / `PlaylistDetail` headers use `deleteSourceDescription`.
- [ ] 2.46 Add a "Favorites" bullet to README.md's Main Features (kept out of rotation, one place to find them); verify the link-free sentence reads with the neighbouring bullets.

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 `SqliteVideoRepository::it_should_round_trip_a_favorite_video`: `favorited_at` round-trips, `None` included.
- [ ] 3.2 `SqliteVideoRepository::it_should_list_only_favorite_videos`.
- [ ] 3.3 `SqliteChannelVideoRepository::it_should_round_trip_a_kept_channel_video`.
- [ ] 3.4 `SqlitePlaylistVideoRepository::it_should_round_trip_a_kept_playlist_video`.

## 4. Verification

- [ ] 4.1 Run `scripts/run-tests.sh --lint --web --rust`; all suites pass.
- [ ] 4.2 Manual (agent):
  - Start `scripts/run-local.sh --port <free port> --fresh-videos`, seed a channel with `video_limit` 2 via the HTTP API, and wait for its videos to download.
  - With the Playwright MCP browser, star a video in the detail pane. Assert:
    - the `PUT` request was sent
    - the card shows a star
    - the sidebar "Favorites" row opens `/favorites` listing it
  - Lower the channel's `video_limit` to 1 via the API and force a sync. Assert:
    - the favorite stays listed with a "Kept" badge, after the other video
    - its file is still in the fresh videos dir
  - Unfavorite it, assert the confirmation text, confirm, and force a sync. Assert:
    - it left the channel list
    - its files are gone
  - Open the channel delete confirmation with one favorite and assert the favorites sentence. Take screenshots.
  - Stop the server, delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.3 Manual (human): on Plex, check that a kept favorite's episode stays in the show after it ages out of its channel's window. Unfavorite it, then check that it leaves the show after the next sync.
