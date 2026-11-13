# Tasks

## 1. Walking skeleton

- [x] 1.0 Extract `PlexEpisodeFinder` (refactor, no behaviour change): move `PlexLibrary` and `PlexLabelSyncer`'s `read_library` / `served_sections` / `episodes_by_file` / `find_episode` into `domain/services/plex_episode_finder.rs` (`PlexEpisodeFinder` + `PlexEpisodeFinderApi`); `PlexLabelSyncer::new` takes it in place of `video_owner_finder`, `videos_path` and `plex_videos_path`; `serve.rs` builds one. Done when the existing `sync_plex_labels_task` suite passes unchanged, plus `cargo fmt --check` and clippy.

- [x] 1.1 Create every file, type and signature from design.md's Types & Signatures: migration `0031_episode_watch.sql` registered in `sqlite_migrations.rs`; `WatchProgress` (`domain/video/watch_progress.rs`); `WatchState::progress` / `changed_at` / `adopt` and `Video::adopt_watch_progress`; `PlexWatchState` and `PlexEpisode.watch` (`list_episodes` filling it with `PlexWatchState::unplayed()` for now); `domain/plex_watch` (`EpisodeWatch`, `WatchSide`, `MergeOutcome`, `merge`); `VideoWatchStateUpdaterApi::adopt`; `PlexWatchSyncer` + `PlexWatchSyncerApi` with `SYNC_INTERVAL` / `BATCH_SIZE`; `Task::SyncPlexWatchState` (`sync_plex_watch_state`, empty payload, decode, `TaskLane::Plex`); `PlexLibraryRepository::mark_watched` / `mark_unwatched` / `save_position` (HTTP bodies `Ok(())`) and `FakePlexLibraryRepository::with_episode_watch` plus recorded mutations; `EpisodeWatchRepository` + `SqliteEpisodeWatchRepository` (stub bodies) built in the infrastructure container; `SyncPlexWatchStateTask` registered in `tasks::registry` when a syncer is given; `serve.rs` builds the syncer with the shared `PlexEpisodeFinder` only when Plex is configured, and `prepare_task_queue` requests a watch sync at startup; `web/src/lib/tasks.ts` describes `sync_plex_watch_state`. Bodies return trivial values (`Ok(())`, empty `Vec`, `WatchProgress::Unplayed`, `MergeOutcome::InSync(..)`), never `todo!()`. Done when `cargo build` succeeds, `npm run check` passes and all existing tests pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_push_a_video_watched_in_yarrtube` — watched video, unplayed episode ⇒ `scrobble:{key}`, row agreed Watched.
- [x] 2.2 `it_should_pull_an_episode_watched_in_plex` — episode viewed once ⇒ video watched, position 0.
- [x] 2.3 `it_should_push_a_playback_position` — video unwatched at 300s ⇒ `progress:{key}:300000`.
- [x] 2.4 `it_should_pull_a_playback_position` — episode offset 215540 ms viewed at T ⇒ video unwatched at 215s, last played T.
- [x] 2.5 `it_should_unwatch_an_episode_before_saving_a_rewatch_position` — video unwatched at 600s, episode watched ⇒ `unscrobble` then `progress:{key}:600000`.
- [x] 2.6 `it_should_pull_an_episode_unwatched_in_plex` — agreed Watched, episode unplayed ⇒ video unwatched at 0.
- [x] 2.7 `it_should_push_a_video_unplayed_after_being_watched` — agreed Watched, video unwatched at 0 ⇒ `unscrobble`.
- [x] 2.8 `it_should_bring_every_copy_in_line_with_a_plex_change` — channel + playlist copies, channel episode watched in Plex ⇒ both copies watched and the playlist episode scrobbled in one sync.
- [x] 2.9 `it_should_not_echo_a_pushed_change` — a second sync after a push makes no mutation and no video change.
- [x] 2.10 `it_should_keep_the_most_recently_played_side_when_both_changed` — Plex later ⇒ pull; yarrtube later ⇒ push.
- [x] 2.11 `it_should_seed_from_whichever_side_was_played_on_the_first_sync` — no rows: Plex-only watch pulled, yarrtube-only watch pushed, both unplayed ⇒ nothing.
- [x] 2.12 `it_should_count_a_short_rewatch_in_plex_as_still_watched` — viewed once, offset 5% of duration ⇒ no change, no mutation.
- [x] 2.13 `it_should_skip_a_video_plex_has_not_imported` — no episode for the file ⇒ no mutation, no row.
- [x] 2.14 `it_should_push_to_an_episode_plex_reimported` — row agreed Watched under another rating key, new episode unplayed ⇒ `scrobble` on the new key, video stays watched.
- [x] 2.15 `it_should_forget_a_video_no_longer_downloaded` — its row is deleted.
- [x] 2.16 `it_should_push_the_others_if_plex_rejects_one` — other row agreed; rejected row unchanged.
- [x] 2.17 `it_should_retry_later_if_plex_is_unreachable` — `failing()` ⇒ no change, next `sync_plex_watch_state` at now + 5 min.
- [x] 2.18 `it_should_schedule_the_next_sync_in_five_minutes`
- [x] 2.19 `it_should_schedule_the_next_sync_now_if_pushes_exceeded_the_batch` — BATCH_SIZE + 1 pushes ⇒ BATCH_SIZE mutations, next sync at now.
- [x] 2.20 `web/src/lib/tasks.test.ts`: `describes a Plex watch sync` — "Syncing Plex watch state" under **Other** (Vitest, `npm run test`).

## 3. Infrastructure adapters (TDD)

`PlexLibraryRepository` (`mockito`)

- [x] 3.1 `it_should_list_an_episodes_watch_state` — `viewCount`, `viewOffset`, `lastViewedAt`, `duration` ⇒ `PlexWatchState`.
- [x] 3.2 `it_should_list_an_unplayed_episode` — fields absent ⇒ `PlexWatchState::unplayed()`.
- [ ] 3.3 `it_should_mark_an_episode_watched` — `GET /:/scrobble` with `identifier` and `key`, token header.
- [ ] 3.4 `it_should_mark_an_episode_unwatched` — `GET /:/unscrobble`.
- [ ] 3.5 `it_should_save_an_episode_position_in_milliseconds` — `GET /:/progress` with `time=215000&state=stopped`.

`SqliteEpisodeWatchRepository`

- [ ] 3.6 `it_should_round_trip_each_agreed_progress` — Unplayed, InProgress(215), Watched.
- [ ] 3.7 `it_should_replace_a_row_on_save`
- [ ] 3.8 `it_should_delete_rows_by_video_id`

## 4. Verification

- [ ] 4.1 Document the watch sync in `doc/PLEX.md` (what syncs both ways, every 5 min, token user only, first-sync seeding, most recent change wins, short rewatches) and mention it next to labels in `doc/INSTALLATION.md`'s Plex variables; verify both read correctly against the specs.
- [ ] 4.2 Run `scripts/run-tests.sh --lint --web --rust`; all pass.
- [ ] 4.3 Manual (human), against a real Plex server with the integration configured:
  - watch a video to the end in a Plex client ⇒ within 5 min it shows watched in the web UI;
  - stop a video halfway in a Plex client ⇒ within 5 min it is under "Continue watching" in the web UI at that position;
  - mark a video watched in the web UI ⇒ within 5 min it is watched in Plex and gone from its category smart playlists;
  - stop a video halfway in the web UI ⇒ within 5 min Plex offers to resume it there;
  - a video tracked by a channel and a playlist, watched in Plex in one show ⇒ both shows' episodes are watched in Plex after the next sync;
  - the tasks view lists "Syncing Plex watch state" under **Other**.
