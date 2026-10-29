# Tasks

## 1. Walking skeleton

- [x] 1.0 Extract `VideoOwnerFinder` (refactor, no behaviour change): move `VideoOwner` to `domain/video/video_owner.rs` (pub) and `VideoReconciler`'s owner lookup into `domain/services/video_owner_finder.rs`; `VideoReconciler` holds it instead of the repositories only that lookup used; `serve.rs` builds one and passes it. Done when the existing `sync_video_task.rs`, `reconcile_channel_task.rs` and `reconcile_playlist_task.rs` suites pass unchanged, plus fmt and clippy.

- [x] 1.1 Create every file, type and signature from design.md: migration `0015_plex_labels.sql` registered in `sqlite_migrations.rs`; `PlexSection.title` + `serves`; `PlexEpisode`, `PlexPlaylist`, `PlexLabels`; `domain/plex_label` (`EpisodeLabels`, `CategoryPlaylist`); `Task::SyncPlexLabels` + decode + `TaskLane::Plex`; new `PlexLibraryRepository` methods (HTTP bodies returning empty/`Ok(())`, `list_sections` parsing `title`) and `FakePlexLibraryRepository` state (episodes with labels, playlists, per-rating-key failure, mutations); `EpisodeLabelsRepository` / `CategoryPlaylistRepository` + SQLite adapters (stub bodies) built in the infrastructure container; `PlexLabelSyncer` + `PlexLabelSyncerApi` (bodies `Ok(())`); `SyncPlexLabelsTask`, both subscribers; `subscribers::registry` gains `Option<PlexLabelSyncer>`; `serve.rs` wiring (`PlexIntegration: Clone`, syncer built with the shared `VideoOwnerFinder` and passed to the task registry and `event_subscribers` only when Plex is configured, `prepare_task_queue` gains `Option<&PlexLabelSyncer>` and calls `request_sync`); `web/src/lib/tasks.ts` entry. Done when `cargo build` succeeds, `npm run check` passes and all existing tests pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_label_the_episode_of_a_categorized_video` — a dirty record's episode gets its categories' names and the row is synced with the rating key.
- [x] 2.2 `it_should_label_an_episode_with_every_category` — all names written.
- [x] 2.3 `it_should_clear_the_labels_of_an_uncategorized_video` — yarrtube owns labels: others removed, field unlocked.
- [x] 2.4 `it_should_not_rewrite_labels_already_in_sync` — case-insensitive comparison, no Plex write.
- [x] 2.5 `it_should_replace_the_label_of_a_renamed_category` — old name removed, new written.
- [x] 2.6 `it_should_keep_a_video_dirty_until_plex_imports_it` — no episode for the file ⇒ stays dirty, no error.
- [x] 2.7 `it_should_keep_a_video_dirty_until_it_is_downloaded`
- [x] 2.8 `it_should_forget_a_deleted_video` — row deleted.
- [x] 2.9 `it_should_label_categorized_videos_never_synced` — backfill of records without a row.
- [x] 2.10 `it_should_relabel_an_episode_plex_reimported` — drift: unknown rating key ⇒ dirty ⇒ labelled under the new key.
- [x] 2.11 `it_should_label_the_others_if_plex_rejects_one` — per-episode failure isolated.
- [x] 2.12 `it_should_keep_videos_dirty_if_plex_is_unreachable` — sync succeeds, next sync in 15 min.
- [x] 2.13 `it_should_schedule_the_next_sync` — periodic chain at now + 15 min.
- [x] 2.14 `it_should_schedule_the_next_sync_now_if_the_batch_was_full`
- [x] 2.15 `it_should_rotate_videos_waiting_for_plex` — least recently checked first.
- [x] 2.16 `it_should_create_a_smart_playlist_for_a_category` — label/section/unwatched/newest-first filter; row stored.
- [x] 2.17 `it_should_title_playlists_by_library_if_a_category_spans_libraries`
- [x] 2.18 `it_should_delete_the_playlist_of_a_category_without_labelled_episodes`
- [x] 2.19 `it_should_replace_the_playlist_of_a_renamed_category`
- [x] 2.20 `it_should_recreate_a_playlist_deleted_in_plex`
- [ ] 2.21 `it_should_not_touch_playlists_it_did_not_create`
- [ ] 2.22 `it_should_mark_the_videos_and_request_a_sync_shortly` — `VideoCategoriesChanged` subscriber; pending at now + `CATEGORY_CHANGE_DELAY` (10 s).
- [ ] 2.23 `it_should_bring_forward_a_pending_sync` — still one pending sync, moved to now + 10 s.
- [ ] 2.23b `it_should_collapse_category_changes_into_one_sync` — two events (as a reconcile pass publishes, one per repaired video) ⇒ one pending sync at now + 10 s.
- [ ] 2.24 `it_should_mark_the_video_and_request_a_sync_in_a_minute` — `VideoDownloaded` subscriber.
- [ ] 2.25 `it_should_request_the_label_sync_at_startup_only_if_plex_is_configured` — `serve::prepare_task_queue`.
- [x] 2.26 `it_should_run_label_syncs_in_the_plex_lane` — `domain/task/task.rs`.
- [x] 2.27 `it_should_equal_labels_ignoring_case` — `PlexLabels`.
- [x] 2.28 `it_should_list_labels_missing_from_another_set_ignoring_case` — `PlexLabels`.
- [x] 2.29 `it_should_keep_each_label_once_ignoring_case` — `PlexLabels`.
- [x] 2.30 `it_should_serve_a_root_under_or_above_its_location` — `PlexSection`.
- [ ] 2.31 `describes a Plex label sync in plain language` — `web/src/lib/tasks.test.ts`.
- [ ] 2.32 `puts Plex label syncs in the other family` — `web/src/lib/tasks.test.ts`.

## 3. Infrastructure adapters (TDD)

HttpPlexLibraryRepository (mockito):
- [x] 3.1 `it_should_list_sections_with_their_titles`
- [x] 3.2 `it_should_read_the_server_id`
- [x] 3.3 `it_should_list_a_section_episodes_with_their_files` — two pages, multi-part media.
- [x] 3.4 `it_should_read_an_episode_labels`
- [x] 3.5 `it_should_save_labels_removing_the_others` — exact query: `label[i].tag.tag`, `label[].tag.tag-=`, `label.locked=1`.
- [x] 3.6 `it_should_clear_labels_and_unlock_them` — `label.locked=0`.
- [x] 3.7 `it_should_list_playlists`
- [x] 3.8 `it_should_create_a_smart_playlist_filtered_by_label` — exact `uri`.
- [x] 3.9 `it_should_delete_a_playlist`

SqliteEpisodeLabelsRepository:
- [x] 3.10 `it_should_round_trip_an_episode_labels_row`
- [x] 3.11 `it_should_list_dirty_rows_least_recently_checked_first_up_to_a_limit`
- [x] 3.12 `it_should_list_only_synced_rows`

SqliteCategoryPlaylistRepository:
- [x] 3.13 `it_should_save_list_and_delete_category_playlists`
- [x] 3.14 `it_should_key_playlists_by_label_ignoring_case`

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` and `npm run check` (in `web/`) all pass.
- [ ] 4.2 End-to-end suite passes via `scripts/run-tests.sh` (no Plex configured: nothing changes).
- [ ] 4.3 Update `doc/PLEX.md`: what labels and playlists yarrtube manages, that episode labels are hidden in Plex Web, one playlist per library, playlists belong to the token's user, the 15-minute sync, the label-restriction caveat for shared users. Verified by reading it against spec.md.
- [ ] 4.4 Manual (agent): on a copy of the user's database (copy `yarrtube.sqlite3` to a temp path, never use the original), start `scripts/run-local.sh --port <free port>` with the Plex variables set against the user's Plex server, create a throwaway category, add one small channel already downloaded and imported, and confirm via `GET /library/sections/{id}/all?type=4&episode.label=<name>` that its episodes are labelled and via `GET /playlists?playlistType=video` that the smart playlist exists and lists them. Rename the category and confirm the label and playlist follow; delete it and confirm labels and playlist are gone. Stop the server and delete the copy.
- [ ] 4.5 Manual (human): in a Plex client (web and a TV app), open the category playlist, play it, mark an episode watched and confirm it leaves the playlist; confirm a managed/shared user with label restrictions still sees the episodes.
