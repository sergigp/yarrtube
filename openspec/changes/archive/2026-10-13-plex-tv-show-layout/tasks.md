## 0. Spike (manual, before any code)

- [x] 0.1 On the real Plex server, create a scratch TV Shows library (Plex NFO Series agent) over a folder holding `Test Show/tvshow.nfo`, `poster.jpg` and `Season 2026/S2026E031501 - One.{mp4,nfo,jpg}` plus an 8-digit variant; verify both episodes appear with the NFO title and the right numbers, then mark one watched, rename its three files to `S2026E031502`, rescan and note whether the watched flag follows the number; record the outcome in design.md "Risks" and stop if six digits fail

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md across all layers, wired end-to-end: migration `0008_episode_numbers.sql` registered; `EpisodeNumber`, `Video::episode_number`, `episode_base_name`/`episode_relative_path`, layout helpers, `Show`, `render_episode_nfo`/`render_show_nfo`; `ShowFileWriter`, `VideoRelocator`, `LayoutMigrator` with their traits; new port signatures on `VideoDownloaderRepository`, `VideoFileRepository`, `ShowFileRepository`, `VideoRepository`, `VideoMetadataRepository`, `ChannelAvatarRepository`; `PlexLibraryRepository` (renamed, trimmed) and `PlexFolderScanner` without section ids; collections task, reconciler, deleter, deletion subscribers, `domain/plex` collection types and the `ReconcilePlexCollections` task variant deleted, registries and `serve.rs` wiring updated (`PlexIntegration` = URL + token + videos path, `.downloads` created at startup); `MigrateLayout` subcommand dispatched from `main.rs`; fakes for every new or changed port; bodies return trivial values. Done when `cargo build` succeeds and all existing tests pass (tests of deleted code removed, tests of changed signatures adapted mechanically)

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_save_a_downloaded_video_as_an_episode_in_its_season_folder` — download task places media/jpg/nfo at `Season 2026/S2026E031501 - Title.*`, records relative filenames and the number, removes staging, publishes `VideoDownloaded` with the show dir
- [x] 2.2 `it_should_write_the_episode_nfo_before_moving_the_media_into_place` — fake file repository order shows the nfo write before the media move
- [x] 2.3 `it_should_number_two_same_day_videos_of_one_show_consecutively` — second same-day download gets `031502`
- [x] 2.4 `it_should_reuse_the_number_recorded_by_the_thumbnail_fetch` — download lands next to the prefetched jpg under the same base name
- [x] 2.5 `it_should_number_from_the_upload_date_when_no_timestamp_is_reported` — number derived from `upload_date` only
- [x] 2.6 `it_should_download_under_its_episode_name_without_metadata` — Data API failing twice: file placed, no nfo, no metadata row
- [x] 2.7 `it_should_discard_the_staging_dir_when_the_download_fails` — staging gone, season folder untouched
- [x] 2.8 `it_should_discard_the_download_when_the_video_was_deleted_meanwhile` — staging gone, nothing recorded, show dir untouched
- [x] 2.9 `it_should_place_a_prefetched_thumbnail_as_the_episode_image` — thumbnail task: jpg at episode path, number and thumbnail recorded, status untouched
- [x] 2.10 `it_should_leave_the_video_untouched_when_the_thumbnail_fetch_fails` — staging removed, no number, no thumbnail
- [x] 2.11 `it_should_write_the_show_files_when_a_channel_is_created` — `tvshow.nfo` with name and channel id, `poster.jpg` equal to the avatar
- [x] 2.12 `it_should_write_the_show_nfo_without_a_poster_for_a_playlist` — playlist created: nfo with playlist id, no poster
- [x] 2.13 `it_should_rewrite_missing_show_files_on_a_recurring_reconcile` — reconcile channel task rewrites a removed `tvshow.nfo`
- [x] 2.14 `it_should_not_sweep_show_files_season_folders_or_episode_files_of_recorded_videos` — only the stray file in the season folder is deleted
- [x] 2.15 `it_should_protect_files_sharing_the_base_name_of_an_in_progress_numbered_video` — nfo of an in-progress numbered video kept
- [x] 2.16 `it_should_relocate_a_legacy_video_when_its_metadata_is_recovered` — legacy folder moved into the season folder, `movie.nfo` removed, empty folder removed, filenames and number recorded from `published_at`
- [x] 2.17 `it_should_leave_a_legacy_video_in_place_when_its_metadata_is_still_unavailable` — no move, no reset
- [x] 2.18 `it_should_reset_a_numbered_video_whose_file_is_missing_without_dropping_its_number` — PENDING, filenames cleared, number kept
- [x] 2.19 `it_should_delete_an_episodes_media_thumbnail_and_nfo` — delete task removes the three files, show files untouched
- [x] 2.20 `it_should_delete_a_legacy_per_video_folder` — delete task removes the whole legacy folder
- [x] 2.21 `it_should_scan_the_show_folder_in_every_section_containing_it` — scan subscriber scans `output_dir` in the containing section only
- [x] 2.22 `it_should_skip_the_scan_when_no_section_contains_the_show_folder` — warning logged, `Ok`
- [x] 2.23 `it_should_register_the_scan_subscriber_only_with_url_token_and_videos_path` — serve wiring with and without each variable, replacing the section-id tests
- [x] 2.24 `it_should_migrate_a_legacy_library_into_the_tv_layout` — migrate-layout over two sources and three videos (one `[id]`-suffixed): files moved, rows updated, show files written, old folders gone, report lists three moves
- [x] 2.25 `it_should_number_a_same_day_batch_in_publish_time_order` — three same-day videos get `01..03` by `published_at`
- [x] 2.26 `it_should_skip_videos_without_metadata_or_without_a_file` — both skips reported with reasons, nothing moved for them
- [x] 2.27 `it_should_change_nothing_on_a_dry_run` — report equals the real plan (a same-day batch planned `01..03`), filesystem and rows untouched
- [x] 2.28 `it_should_move_nothing_on_a_second_run` — zero moves, show files rewritten
- [x] 2.29 `it_should_report_a_failed_move_and_continue` — one failing move: that row unchanged, others moved, one failure in the report
- [x] 2.30 `it_should_fail_if_a_video_failed_to_move` — CLI returns a failure exit code
- [x] 2.31 `it_should_not_register_the_collections_task_or_subscribers` — handler registry has no `reconcile_plex_collections`, no deletion subscribers

## 3. Infrastructure adapters (TDD)

- [x] 3.1 ytdlp: `it_should_download_into_the_staging_dir_with_the_video_id_as_name` — `-o "%(id)s.%(ext)s"`, cwd = staging dir
- [x] 3.2 ytdlp: `it_should_report_the_publish_timestamp_printed_by_ytdlp` — `--print %(timestamp)s` parsed to UTC
- [x] 3.3 ytdlp: `it_should_fall_back_to_the_upload_date_when_the_timestamp_is_na` — `upload_date` → midnight UTC
- [x] 3.4 ytdlp: `it_should_report_the_written_thumbnail_path` — thumbnail fetch returns the staged jpg path and timestamp
- [x] 3.5 ytdlp: `it_should_report_the_staged_thumbnail_written_next_to_the_download` — after a successful download, `thumbnail_path` is the `<media stem>.jpg` in the staging dir when present, `None` otherwise; also rewrite and un-ignore the download task's `it_should_detect_a_real_thumbnail_file_written_by_yt_dlp_alongside_the_video` (real fake-yt-dlp script, episode paths)
- [x] 3.6 FilesystemVideoFileRepository: `it_should_list_top_level_entries_and_season_files` — `Season 2026/<file>` entries plus legacy top-level entries, temp files excluded
- [x] 3.7 FilesystemVideoFileRepository: `it_should_move_a_file_creating_its_parent` — rename into a not-yet-existing season folder
- [x] 3.8 FilesystemVideoFileRepository: `it_should_remove_an_empty_dir_only` — non-empty dir left alone
- [x] 3.9 FilesystemVideoFileRepository: `it_should_delete_a_file_or_a_dir_by_relative_path` — episode file and legacy folder
- [x] 3.10 FilesystemShowFileRepository: `it_should_write_the_show_nfo_and_copy_the_poster` — bytes equal the source avatar
- [x] 3.11 FilesystemShowFileRepository: `it_should_overwrite_an_existing_poster`
- [x] 3.12 SqliteVideoRepository: `it_should_assign_the_first_index_of_the_day` — `031501`
- [x] 3.13 SqliteVideoRepository: `it_should_assign_the_next_free_index_within_the_same_show_and_day` — sibling via `playlist_videos` and via `channel_videos`
- [x] 3.14 SqliteVideoRepository: `it_should_number_copies_in_different_shows_independently`
- [x] 3.15 SqliteVideoRepository: `it_should_return_the_existing_number_when_already_assigned`
- [x] 3.16 SqliteVideoRepository: `it_should_persist_and_read_the_episode_number` — round trip through `save`/`find`/`update`
- [x] 3.17 SqliteVideoRepository: `it_should_list_downloaded_videos`
- [x] 3.18 SqliteVideoMetadataRepository: `it_should_write_an_episode_nfo_with_season_episode_and_aired` — `<episodedetails>` with `season`, `episode`, `aired`, `uniqueid`, escaped text
- [x] 3.19 SqliteVideoMetadataRepository: `it_should_write_the_nfo_at_the_given_path`
- [x] 3.20 SqliteVideoMetadataRepository: `it_should_remove_the_nfo_at_the_given_path`
- [x] 3.21 sqlite_migrations: `it_should_add_the_episode_number_columns`
- [x] 3.22 HttpPlexLibraryRepository: keep the `list_sections`/`scan_path` mockito tests green after trimming the trait, delete the collection tests
- [x] 3.23 FilesystemChannelAvatarRepository: `it_should_resolve_the_path_of_a_stored_avatar`
- [x] 3.24 web: `storageLocation.test.ts` (season folder from an episode filename, base path when no file) and `VideoDetail.test.tsx` `shows the season folder as the storage location`; remove the `reconcile_plex_collections` label and its tests; `npm run check` passes
- [x] 3.25 SqliteVideoRepository: `it_should_peek_the_next_episode_number_without_recording_it`

## 4. Docs, config and verification

- [x] 4.1 Rewrite `doc/PLEX.md` for the TV Shows library (NFO Series agent, Seasons setting, local assets, partial auto-scan, the three scan env vars, the migration procedure with dry run, deleting the old Movies library, watch history caveat, troubleshooting); update `doc/INSTALLATION.md` (env table: remove section ids and reconcile interval, describe `migrate-layout`), `doc/ARCHITECTURE.md`, `README.md`, `CLAUDE.md` ("keeps Plex collections in sync" → TV layout + scan trigger), `.env.example`, `scripts/run-local.sh`; verify by reading them and `bash -n scripts/run-local.sh`
- [x] 4.2 `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo test --locked`, `npm run check` in `web/`; all must pass
- [ ] 4.3 Manual check with `scripts/run-local.sh` on a copy of a legacy `videos/` dir: run `migrate-layout --dry-run` then the real run, start the daemon, add a channel, confirm the show files, a new episode in its season folder, playback in the SPA, and the Plex scan log line when the Plex vars are set

## 5. Playlist episode numbers in playlist order (TDD)

- [x] 5.1 Walking skeleton: split `EpisodeNumber` into `ChannelEpisodeNumber` + `PlaylistEpisodeNumber` behind `enum EpisodeNumber` (design.md decision 10 and Types & Signatures); `Playlist.last_episode` with `next_episode_number`/`with_last_episode`; edit `0008_episode_numbers.sql` in place (`playlists.last_episode`, playlist videos numbered by position); all existing tests compile and pass unchanged
- [x] 5.2 VO: `PlaylistEpisodeNumber` `it_should_reject_episode_zero`
- [x] 5.3 VO: `PlaylistEpisodeNumber` `it_should_give_the_next_number`
- [x] 5.4 VO: `EpisodeNumber` `it_should_render_a_playlist_code_padded_to_three_digits` — `S01E007`, `Season 1`
- [x] 5.5 VO: `EpisodeNumber` `it_should_widen_a_playlist_code_past_999` — `S01E1000`
- [x] 5.6 VO: `EpisodeNumber` `it_should_decode_season_one_as_a_playlist_number`
- [x] 5.7 sqlite_migrations: `it_should_number_existing_playlist_videos_by_position` — ties by insertion, channel videos untouched, `last_episode` = count, `NULL` for an empty playlist
- [x] 5.8 SqlitePlaylistRepository: `it_should_raise_the_last_episode`, `it_should_never_lower_the_last_episode`, `it_should_keep_the_last_episode_when_updating_from_a_stale_read` (were `it_should_persist_the_last_episode`; revised in 7.6)
- [x] 5.9 SqliteVideoRepository: `it_should_persist_and_read_a_playlist_episode_number`
- [x] 5.10 SqliteVideoRepository: same-day siblings via `channel_videos` only — revise 3.13's playlist sibling case so a playlist link no longer counts
- [x] 5.11 `it_should_number_new_playlist_videos_in_playlist_order` — playlist created: positions 0..2 with publish times in another order → episodes 1..3, `last_episode` 3
- [x] 5.12 `it_should_number_a_video_added_to_the_playlist_after_the_last_one` — reconcile playlist task: `last_episode` 5, new item at position 0 → 6
- [x] 5.13 `it_should_not_reuse_the_number_of_a_removed_video` — the video holding 6 leaves, next new item → 7
- [x] 5.14 `it_should_keep_episode_numbers_when_the_playlist_is_reordered` — positions swap, numbers and filenames unchanged
- [x] 5.15 `it_should_save_a_playlist_video_under_its_playlist_episode_number` — download task: `Season 1/S01E007 - Title.*` despite a 2014 publish time
- [x] 5.16 `it_should_move_legacy_playlist_videos_under_their_recorded_numbers` — migrate-layout: publish order reversed, moved by position number
- [x] 5.17 `it_should_move_a_playlist_video_without_metadata` — migrate-layout: numbered, no metadata → moved without NFO, not skipped

## 6. Docs and verification for playlist numbering

- [x] 6.1 Update `doc/PLEX.md` (layout example and numbering paragraph for playlists), `doc/INSTALLATION.md` and `doc/ARCHITECTURE.md` "Videos on disk", `CLAUDE.md` layout line
- [x] 6.2 `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo test --locked`, `npm run check` in `web/`; all must pass

## 7. Audit fixes

- [x] 7.1 Mobile sidebar: re-measure its top while open via `ResizeObserver` on the announcement slot (`web/src/App.tsx`); smoke `mobile sidebar stays closable when the announcement appears after it opened`
- [x] 7.2 Staging dir keyed by video record id; the download and thumbnail tests assert `.downloads/<record id>`
- [x] 7.3 Thumbnail fetch discards its staging dir after success too; `it_should_place_a_prefetched_thumbnail_as_the_episode_image` asserts it
- [x] 7.4 `VideoRelocator` completes a half-moved set: `it_should_complete_a_video_whose_media_an_interrupted_run_already_moved`
- [x] 7.5 Show files best effort: `it_should_sync_the_channel_even_if_its_show_files_cannot_be_written`, `it_should_report_show_files_it_cannot_write_and_still_move_the_videos`
- [x] 7.6 Playlist counter written only by `PlaylistRepository::record_last_episode` (never lowered); `update` no longer writes it
- [x] 7.7 Dry run without a database copy: `snapshot` removed, `migrate_layout::run` migrates the schema then plans; `doc/PLEX.md`, `doc/INSTALLATION.md` updated
- [x] 7.8 Dry-run numbering tests: `it_should_plan_the_same_day_of_different_years_independently_on_a_dry_run`, `it_should_plan_past_a_later_number_already_recorded_that_day_on_a_dry_run`
- [x] 7.9 `0008` deletes queued `reconcile_plex_collections` tasks: `it_should_drop_queued_plex_collection_reconciles_when_migrating`
- [x] 7.10 `assign_episode_number` fails for an unnumbered video in no channel: `it_should_fail_to_number_an_unnumbered_playlist_video_by_publish_date`; repository tests rewritten around channel siblings
- [x] 7.11 `ChannelEpisodeNumber::new` validates season, MMDD and index 01..=99, with value-object tests
- [x] 7.12 `EpisodePaths` replaces `episode_base_name` / `episode_relative_path` at every call site; `season_folder()` uses `SEASON_FOLDER_PREFIX`
- [x] 7.13 `FilesystemShowFileRepository` skips unchanged writes: `it_should_leave_unchanged_show_files_untouched`, `it_should_overwrite_a_changed_show_nfo`
- [x] 7.14 Tests wire `VideoRelocator` with `FixedClock`; misplaced doc comments in `playlist_video_reconciler.rs` fixed
- [x] 7.15 `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo test --locked`, `npm run check` in `web/`, `scripts/run-smoke-tests.sh` (24/24); `cargo mutants` over the fixes: no missed mutants

## 8. Second audit fixes

- [x] 8.1 A finished download that cannot be placed (numbering, move or record failing) discards its staging dir and errors the video like a failed download, keeping an assigned number: `it_should_mark_errored_if_the_download_cannot_be_moved_into_place_on_the_last_attempt`
- [x] 8.2 Strict layout detection: `Season <digits>` folders and `S<digits>E<digits> - ` episode files only: `it_should_recognize_an_episode_file_in_its_season_folder`, `it_should_treat_a_legacy_folder_titled_like_a_season_as_legacy`, `it_should_only_take_a_season_and_number_as_a_season_folder`
- [x] 8.3 Audit automatic lanes: behaviour-preserving cleanups (convention/simplification) and mutant-killing tests
- [x] 8.4 `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo test --locked`
