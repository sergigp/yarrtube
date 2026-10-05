## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures), wired end-to-end with trivial bodies:
  - Backend:
    - `was_watched` param on `Video::update_watch_state` and `VideoWatchStateUpdaterApi::update`, ignored for now
    - `UpdateWatchStateError::VideoNotDownloaded` mapped to 400
    - `mark_video_watched` on the updater returning `Ok(())`
    - `videos::mark_video_watched` handler returning 204, routed at `POST /videos/{id}/watched`
    - `RecordProgressRequest.was_watched: Option<bool>`, not yet required
    - `MISSING_WAS_WATCHED`
  - Existing tests pass `was_watched` matching the video's state.
  - Web:
    - `VideoProgress.was_watched` (the hook sends the attach-time `watched` for now)
    - `markVideoWatched` client
    - `useMarkVideoWatched`
    - `VideoActionsMenu` rendering a ⋮ trigger with a "Mark as watched" item that does nothing yet. It is not placed in any view yet.

  Done when `cargo build` succeeds and `cargo test --locked` and `npm run check` pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_mark_a_video_watched`: marking a downloaded unwatched video responds 204 and leaves it watched, position 0, `watched_at` = clock
- [x] 2.2 `it_should_mark_every_copy_of_a_video_watched`: channel and playlist copies both become watched
- [x] 2.3 `it_should_not_change_the_last_played_time_when_marking_a_video_watched`: `last_played_at` untouched
- [x] 2.4 `it_should_leave_an_already_watched_video_unchanged_when_marking_it_watched`: 204, `watched_at` keeps the earlier time
- [x] 2.5 `it_should_fail_to_mark_watched_a_video_not_downloaded`: 400 when no copy is `Downloaded`, nothing changes
- [ ] 2.6 `it_should_fail_to_mark_watched_an_unknown_video`: 400 for an untracked YouTube ID
- [ ] 2.7 `it_should_fail_to_mark_watched_if_invalid_video_id_provided`: 400 for an invalid ID
- [ ] 2.8 `it_should_ignore_a_stale_progress_report_on_a_video_marked_watched`: `was_watched: false` at 40% on a watched video responds `watched: true`, video unchanged including `last_played_at`
- [ ] 2.9 `it_should_fail_to_record_progress_if_was_watched_missing`: 400 and nothing recorded when `was_watched` is absent
- [ ] 2.10 `VideoActionsMenu` "marks the video watched when "Mark as watched" is chosen": POST routed, channels and home refetched
- [ ] 2.11 `VideoActionsMenu` "disables "Mark as watched" when not markable"
- [ ] 2.12 `VideoActionsMenu` "alerts and leaves the video as it was when marking fails"
- [ ] 2.13 `Home` "marking a continue-watching video watched removes it from the section": menu placed right of the card title block
- [ ] 2.14 `ChannelDetail` "opening a row's menu does not change the selected video": `VideoListPane` row split into a select button plus a sibling menu
- [ ] 2.15 `ChannelDetail` "marking a row watched shows its tick"
- [ ] 2.16 `VideoDetail` "marks the selected video watched from the detail pane": menu in the title row
- [ ] 2.17 `useWatchProgress` "reports was_watched as the session's state"
- [ ] 2.18 `useWatchProgress` "pauses, rewinds and reports nothing when the video becomes watched while loaded": unmounting after the flip sends no report
- [ ] 2.19 `useWatchProgress` "reports was_watched true after a flip once playback moves on"

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 None. No adapter changes (design.md Test Plan §2); verify by confirming no file under `src/infrastructure/` is in the diff.

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass
- [ ] 4.2 `npm run check` passes in `web/`
- [ ] 4.3 Manual check with `scripts/run-local.sh`:
  - (a) Open a "Continue watching" video, mark its channel watched, navigate home. The video is gone and the badge is 0.
  - (b) Mark a playing video watched from the detail pane, then leave. It stays watched.
  - (c) Mark a video watched from a home card. It leaves "Continue watching".
  - (d) The ⋮ menu is disabled for watched and not-downloaded videos.
