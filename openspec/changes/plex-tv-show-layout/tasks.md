## 0. Spike (manual, before any code)

- [ ] 0.1 On the real Plex server, create a scratch TV Shows library (Plex NFO Series agent) over a folder holding `Test Show/tvshow.nfo`, `poster.jpg` and `Season 2026/S2026E031501 - One.{mp4,nfo,jpg}` plus an 8-digit variant; verify both episodes appear with the NFO title and the right numbers, then mark one watched, rename its three files to `S2026E031502`, rescan and note whether the watched flag follows the number; record the outcome in design.md "Risks" and stop if six digits fail

## 1. Walking skeleton

- [ ] 1.1 Create every file, type and signature from design.md across all layers, wired end-to-end: migration `0008_episode_numbers.sql` registered; `EpisodeNumber`, `Video::episode_number`, `episode_base_name`/`episode_relative_path`, layout helpers, `Show`, `render_episode_nfo`/`render_show_nfo`; `ShowFileWriter`, `VideoRelocator`, `LayoutMigrator` with their traits; new port signatures on `VideoDownloaderRepository`, `VideoFileRepository`, `ShowFileRepository`, `VideoRepository`, `VideoMetadataRepository`, `ChannelAvatarRepository`; `PlexLibraryRepository` (renamed, trimmed) and `PlexFolderScanner` without section ids; collections task, reconciler, deleter, deletion subscribers, `domain/plex` collection types and the `ReconcilePlexCollections` task variant deleted, registries and `serve.rs` wiring updated (`PlexIntegration` = URL + token + videos path, `.downloads` created at startup); `MigrateLayout` subcommand dispatched from `main.rs`; fakes for every new or changed port; bodies return trivial values. Done when `cargo build` succeeds and all existing tests pass (tests of deleted code removed, tests of changed signatures adapted mechanically)

## 2. Behaviour (TDD)

- [ ] 2.1 `it_should_save_a_downloaded_video_as_an_episode_in_its_season_folder` — download task places media/jpg/nfo at `Season 2026/S2026E031501 - Title.*`, records relative filenames and the number, removes staging, publishes `VideoDownloaded` with the show dir
- [ ] 2.2 `it_should_write_the_episode_nfo_before_moving_the_media_into_place` — fake file repository order shows the nfo write before the media move
- [ ] 2.3 `it_should_number_two_same_day_videos_of_one_show_consecutively` — second same-day download gets `031502`
- [ ] 2.4 `it_should_reuse_the_number_recorded_by_the_thumbnail_fetch` — download lands next to the prefetched jpg under the same base name
- [ ] 2.5 `it_should_number_from_the_upload_date_when_no_timestamp_is_reported` — number derived from `upload_date` only
- [ ] 2.6 `it_should_download_under_its_episode_name_without_metadata` — Data API failing twice: file placed, no nfo, no metadata row
- [ ] 2.7 `it_should_discard_the_staging_dir_when_the_download_fails` — staging gone, season folder untouched
- [ ] 2.8 `it_should_discard_the_download_when_the_video_was_deleted_meanwhile` — staging gone, nothing recorded, show dir untouched
- [ ] 2.9 `it_should_place_a_prefetched_thumbnail_as_the_episode_image` — thumbnail task: jpg at episode path, number and thumbnail recorded, status untouched
- [ ] 2.10 `it_should_leave_the_video_untouched_when_the_thumbnail_fetch_fails` — staging removed, no number, no thumbnail
- [ ] 2.11 `it_should_write_the_show_files_when_a_channel_is_created` — `tvshow.nfo` with name and channel id, `poster.jpg` equal to the avatar
- [ ] 2.12 `it_should_write_the_show_nfo_without_a_poster_for_a_playlist` — playlist created: nfo with playlist id, no poster
- [ ] 2.13 `it_should_rewrite_missing_show_files_on_a_recurring_reconcile` — reconcile channel task rewrites a removed `tvshow.nfo`
- [ ] 2.14 `it_should_not_sweep_show_files_season_folders_or_episode_files_of_recorded_videos` — only the stray file in the season folder is deleted
- [ ] 2.15 `it_should_protect_files_sharing_the_base_name_of_an_in_progress_numbered_video` — nfo of an in-progress numbered video kept
- [ ] 2.16 `it_should_relocate_a_legacy_video_when_its_metadata_is_recovered` — legacy folder moved into the season folder, `movie.nfo` removed, empty folder removed, filenames and number recorded from `published_at`
- [ ] 2.17 `it_should_leave_a_legacy_video_in_place_when_its_metadata_is_still_unavailable` — no move, no reset
- [ ] 2.18 `it_should_reset_a_numbered_video_whose_file_is_missing_without_dropping_its_number` — PENDING, filenames cleared, number kept
- [ ] 2.19 `it_should_delete_an_episodes_media_thumbnail_and_nfo` — delete task removes the three files, show files untouched
- [ ] 2.20 `it_should_delete_a_legacy_per_video_folder` — delete task removes the whole legacy folder
- [ ] 2.21 `it_should_scan_the_show_folder_in_every_section_containing_it` — scan subscriber scans `output_dir` in the containing section only
- [ ] 2.22 `it_should_skip_the_scan_when_no_section_contains_the_show_folder` — warning logged, `Ok`
- [ ] 2.23 `it_should_register_the_scan_subscriber_only_with_url_token_and_videos_path` — serve wiring with and without each variable, replacing the section-id tests
- [ ] 2.24 `it_should_migrate_a_legacy_library_into_the_tv_layout` — migrate-layout over two sources and three videos (one `[id]`-suffixed): files moved, rows updated, show files written, old folders gone, report lists three moves
- [ ] 2.25 `it_should_number_a_same_day_batch_in_publish_time_order` — three same-day videos get `01..03` by `published_at`
- [ ] 2.26 `it_should_skip_videos_without_metadata_or_without_a_file` — both skips reported with reasons, nothing moved for them
- [ ] 2.27 `it_should_change_nothing_on_a_dry_run` — report equals the real plan, filesystem and rows untouched
- [ ] 2.28 `it_should_move_nothing_on_a_second_run` — zero moves, show files rewritten
- [ ] 2.29 `it_should_report_a_failed_move_and_continue` — one failing move: that row unchanged, others moved, one failure in the report
- [ ] 2.30 `it_should_exit_non_zero_when_a_move_failed` — CLI returns a failure exit code
- [ ] 2.31 `it_should_not_register_the_collections_task_or_subscribers` — handler registry has no `reconcile_plex_collections`, no deletion subscribers

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 ytdlp: `it_should_download_into_the_staging_dir_with_the_video_id_as_name` — `-o "%(id)s.%(ext)s"`, cwd = staging dir
- [ ] 3.2 ytdlp: `it_should_report_the_publish_timestamp_printed_by_ytdlp` — `--print %(timestamp)s` parsed to UTC
- [ ] 3.3 ytdlp: `it_should_fall_back_to_the_upload_date_when_the_timestamp_is_na` — `upload_date` → midnight UTC
- [ ] 3.4 ytdlp: `it_should_report_the_written_thumbnail_path` — thumbnail fetch returns the staged jpg path and timestamp
- [ ] 3.5 FilesystemVideoFileRepository: `it_should_list_top_level_entries_and_season_files` — `Season 2026/<file>` entries plus legacy top-level entries, temp files excluded
- [ ] 3.6 FilesystemVideoFileRepository: `it_should_move_a_file_creating_its_parent` — rename into a not-yet-existing season folder
- [ ] 3.7 FilesystemVideoFileRepository: `it_should_remove_an_empty_dir_only` — non-empty dir left alone
- [ ] 3.8 FilesystemVideoFileRepository: `it_should_delete_a_file_or_a_dir_by_relative_path` — episode file and legacy folder
- [ ] 3.9 FilesystemShowFileRepository: `it_should_write_the_show_nfo_and_copy_the_poster` — bytes equal the source avatar
- [ ] 3.10 FilesystemShowFileRepository: `it_should_overwrite_an_existing_poster`
- [ ] 3.11 SqliteVideoRepository: `it_should_assign_the_first_index_of_the_day` — `031501`
- [ ] 3.12 SqliteVideoRepository: `it_should_assign_the_next_free_index_within_the_same_show_and_day` — sibling via `playlist_videos` and via `channel_videos`
- [ ] 3.13 SqliteVideoRepository: `it_should_number_copies_in_different_shows_independently`
- [ ] 3.14 SqliteVideoRepository: `it_should_return_the_existing_number_when_already_assigned`
- [ ] 3.15 SqliteVideoRepository: `it_should_persist_and_read_the_episode_number` — round trip through `save`/`find`/`update`
- [ ] 3.16 SqliteVideoRepository: `it_should_list_downloaded_videos`
- [ ] 3.17 SqliteVideoMetadataRepository: `it_should_write_an_episode_nfo_with_season_episode_and_aired` — `<episodedetails>` with `season`, `episode`, `aired`, `uniqueid`, escaped text
- [ ] 3.18 SqliteVideoMetadataRepository: `it_should_write_the_nfo_at_the_given_path`
- [ ] 3.19 SqliteVideoMetadataRepository: `it_should_remove_the_nfo_at_the_given_path`
- [ ] 3.20 sqlite_migrations: `it_should_add_the_episode_number_columns`
- [ ] 3.21 HttpPlexLibraryRepository: keep the `list_sections`/`scan_path` mockito tests green after trimming the trait, delete the collection tests
- [ ] 3.22 FilesystemChannelAvatarRepository: `it_should_resolve_the_path_of_a_stored_avatar`
- [ ] 3.23 web: `storageLocation.test.ts` (season folder from an episode filename, base path when no file) and `VideoDetail.test.tsx` `shows the season folder as the storage location`; remove the `reconcile_plex_collections` label and its tests; `npm run check` passes

## 4. Docs, config and verification

- [ ] 4.1 Rewrite `doc/PLEX.md` for the TV Shows library (NFO Series agent, Seasons setting, local assets, partial auto-scan, the three scan env vars, the migration procedure with dry run, deleting the old Movies library, watch history caveat, troubleshooting); update `doc/INSTALLATION.md` (env table: remove section ids and reconcile interval, describe `migrate-layout`), `doc/ARCHITECTURE.md`, `README.md`, `CLAUDE.md` ("keeps Plex collections in sync" → TV layout + scan trigger), `.env.example`, `scripts/run-local.sh`; verify by reading them and `bash -n scripts/run-local.sh`
- [ ] 4.2 `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo test --locked`, `npm run check` in `web/`; all must pass
- [ ] 4.3 Manual check with `scripts/run-local.sh` on a copy of a legacy `videos/` dir: run `migrate-layout --dry-run` then the real run, start the daemon, add a channel, confirm the show files, a new episode in its season folder, playback in the SPA, and the Plex scan log line when the Plex vars are set
