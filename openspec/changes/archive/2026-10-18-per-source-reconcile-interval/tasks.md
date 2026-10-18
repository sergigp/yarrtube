## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures) across all layers: migration 0009 registered, `ReconcileInterval` (constructor accepts any value, `default_interval()` returns 180), new `reconcile_interval` fields on `Channel`/`ChannelView`/`Playlist` with repositories mapping the column, `ScheduledTask::bring_forward` returning `self`, `TaskRepository::find_pending` returning `Ok(None)`, new creator/updater/reconciler signatures (updaters ignore the interval, reconcilers schedule `now + entity.reconcile_interval` from the loaded entity), DTO fields, `serve.rs` wiring without `reconcile_interval_seconds()`; web types, builders, client, `useUpdatePlaylistSettings`, `lib/reconcileInterval.ts` stubs, `ReconcileIntervalField`, `EditPlaylistDialog` rendering nothing, unwired. Done when `cargo build`, `cargo test --locked` and `npm run check` pass with no new tests.

## 2. Behaviour (TDD)

Reconcile tasks:
- [x] 2.1 `reconcile_channel_task::it_should_always_schedule_the_next_reconcile` (updated): next reconcile at `now + 180 min` for a default-interval channel.
- [x] 2.2 `reconcile_channel_task::it_should_schedule_the_next_reconcile_after_the_channels_interval`: 30 min channel → pending task at `now + 30 min`.
- [x] 2.3 `reconcile_channel_task::it_should_not_schedule_the_next_reconcile_if_channel_deleted_during_the_pass`: re-read at scheduling time; `None` → nothing scheduled.
- [x] 2.4 `reconcile_playlist_task::it_should_always_schedule_the_next_reconcile` (updated): `now + 180 min`.
- [x] 2.5 `reconcile_playlist_task::it_should_schedule_the_next_reconcile_after_the_playlists_interval`: 720 min → `now + 720 min`.
- [x] 2.6 `reconcile_playlist_task::it_should_not_schedule_the_next_reconcile_if_playlist_deleted_during_the_pass`.

Channel HTTP:
- [x] 2.7 `it_should_create_a_channel_with_the_default_interval`: response and stored channel have 180.
- [x] 2.8 `it_should_create_a_channel_with_its_interval`: request 30 → response/stored 30.
- [x] 2.9 `it_should_fail_to_create_if_invalid_interval_provided`: 0 → 400 with the VO's exact message.
- [x] 2.10 `it_should_keep_the_interval_of_an_existing_channel_on_duplicate_create`.
- [x] 2.11 `it_should_update_a_channels_interval`: response + stored channel 60, other fields unchanged.
- [x] 2.12 `it_should_bring_the_next_reconcile_forward_if_interval_shortened` (channels): pending at `now + 20h`, update to 60 → `run_at = now + 60 min`, `updated_at = now`.
- [x] 2.13 `it_should_keep_the_next_reconcile_if_interval_lengthened` (channels): pending at `now + 30 min`, update to 1440 → task unchanged.
- [x] 2.14 `it_should_fail_to_update_if_invalid_interval_provided` (channels): 0 → 400.
- [x] 2.15 `it_should_fail_to_update_if_nothing_to_update` (channels, updated): new message naming all three settings.
- [x] 2.16 `it_should_list_channels_with_their_interval`.

Playlist HTTP:
- [x] 2.17 `it_should_create_a_playlist_with_the_default_interval`.
- [x] 2.18 `it_should_create_a_playlist_with_its_interval`.
- [x] 2.19 `it_should_fail_to_create_if_invalid_interval_provided` (playlists).
- [x] 2.20 `it_should_keep_the_interval_of_an_existing_playlist_on_duplicate_create`.
- [x] 2.21 `it_should_update_a_playlists_interval`: `exclude_from_home` absent keeps its value.
- [x] 2.22 `it_should_bring_the_next_reconcile_forward_if_interval_shortened` (playlists).
- [x] 2.23 `it_should_keep_the_next_reconcile_if_interval_lengthened` (playlists).
- [x] 2.24 `it_should_fail_to_update_if_nothing_to_update` (playlists): replaces `it_should_fail_to_update_if_exclude_from_home_missing`.
- [x] 2.25 `it_should_fail_to_update_if_invalid_interval_provided` (playlists).
- [x] 2.26 `it_should_list_playlists_with_their_interval`.

Value object `ReconcileInterval`:
- [x] 2.27 `it_should_accept_one_minute`.
- [x] 2.28 `it_should_reject_zero`: exact message "Reconcile interval must be at least 1 minute (got 0)".
- [x] 2.29 `it_should_reject_a_negative_interval`.
- [x] 2.30 `it_should_accept_a_day` and `it_should_reject_more_than_a_day`: 1440 accepted, 1441 → "Reconcile interval must be at most 1440 minutes (got 1441)".

Web (Vitest):
- [x] 2.31 `lib/reconcileInterval`: `intervalForm` shows 180 as 3 hours and 90 as 90 minutes.
- [x] 2.32 `lib/reconcileInterval`: `intervalMinutes` converts 6 hours to 360, and returns `null` for `''`, `'0'` and `'1.5'`.
- [x] 2.33 `EditChannelDialog`: opens with the interval showing 3 hours; changing only the interval sends `{ reconcile_interval_minutes: 30 }` and no reconcile.
- [x] 2.34 `EditChannelDialog`: an invalid interval flags the field and sends nothing.
- [x] 2.35 `EditPlaylistDialog`: opens titled "Edit Lofi beats settings" showing 3 hours.
- [x] 2.36 `EditPlaylistDialog`: saving 30 minutes sends PATCH `{ reconcile_interval_minutes: 30 }`, no reconcile, and the dialog closes.
- [x] 2.37 `EditPlaylistDialog`: saving unchanged sends nothing; a failed save keeps the dialog open with the error.
- [x] 2.38 `Sidebar`: the playlist row menu lists Sync, Exclude from home, Edit settings, Delete; choosing Edit settings opens the dialog.
- [x] 2.39 `PlaylistDetail`: the header menu offers Edit settings and opens the dialog.
- [x] 2.40 `AddChannelDialog`: submits `reconcile_interval_minutes` 180 by default and 30 when set to 30 minutes under "Advanced options".
- [x] 2.41 `AddPlaylistDialog`: same as 2.40.
- [x] 2.42 Web: `intervalMinutes` returns `null` above 24 hours, and `EditPlaylistDialog` flags more than 24 hours and sends nothing.
- [x] 2.43 `it_should_stamp_the_brought_forward_reconcile_with_the_update_time` (channels): pending task scheduled under an earlier clock → `updated_at` is the update time.
- [x] 2.44 `it_should_stamp_the_brought_forward_reconcile_with_the_update_time` (playlists).

## 3. Infrastructure adapters (TDD)

`SqliteTaskRepository`:
- [x] 3.1 `it_should_find_the_pending_task_with_the_same_type_and_payload`.
- [x] 3.2 `it_should_not_find_a_running_or_completed_task_as_pending`.
- [x] 3.3 `it_should_not_find_a_pending_task_of_another_source`: same type, different payload → `None`.

`SqliteChannelRepository` / `SqlitePlaylistRepository`:
- [x] 3.4 `it_should_round_trip_the_channels_reconcile_interval`.
- [x] 3.5 `it_should_round_trip_the_playlists_reconcile_interval`.

Migrations:
- [x] 3.6 `it_should_give_existing_channels_and_playlists_the_default_interval`: rows inserted before 0009 read back with 180.

## 4. Verification

- [x] 4.1 Remove `YARRTUBE_RECONCILE_INTERVAL_SECONDS` from `doc/INSTALLATION.md`, `doc/ARCHITECTURE.md` and `.env.example`, documenting the per-source setting instead; verify with `grep -rn RECONCILE_INTERVAL_SECONDS` returning nothing outside `openspec/` and `doc/UPGRADING-*`.
- [x] 4.2 Add a smoke test in `smoke-tests/tests/playlist.spec.js` that edits a playlist's interval through the dialog (exercises `PATCH /api/playlists/{id}`).
- [x] 4.3 `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `npm run check` all pass.
- [x] 4.4 Run `scripts/run-smoke-test.sh`, then `scripts/run-local.sh` against an existing database: sources show 3 hours, editing a playlist and a channel interval brings their next reconcile forward in the Tasks view.
