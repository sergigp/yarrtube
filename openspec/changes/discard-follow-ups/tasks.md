# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every type and signature from design.md's Types & Signatures, wired end-to-end, bodies trivial:
  - `Download::restore` and `Video::restore` return `self` / `None`; `Video::is_unavailable` returns `false`.
  - `RestoreDiscardedVideosError` with `Display`, `Error` and `From<anyhow::Error>`.
  - `restore_discarded` on `ChannelVideoReconcilerApi` and `PlaylistVideoReconcilerApi`, returning `Ok(())`, with private `restore_stored_discarded` stubs.
  - `restore_discarded_channel_videos` / `restore_discarded_playlist_videos` handlers answering 204, routed in `src/application/http/mod.rs`.
  - Web: `restoreDiscardedChannelVideos` / `restoreDiscardedPlaylistVideos` in `client.ts`; `useRestoreDiscardedVideos` in `queries.ts`; `DiscardVideoDialog` extracted from `DiscardVideoButton` (copy unchanged); the `discard` prop on `VideoActionsMenu` and the `onRestoreDiscarded` prop on `DetailHeader`, accepted but not rendered yet.

  Done when `cargo build` succeeds and `scripts/run-tests.sh --lint --web --rust` passes. No new behaviour and no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_count_a_discarded_video_toward_the_limit`: the channel window skips only Unavailable videos (`Video::is_unavailable`).
- [x] 2.2 `it_should_request_one_more_upload_per_unavailable_video` replaces `it_should_request_one_more_upload_per_skipped_video`: the listing is widened by the stored Unavailable count only.
- [x] 2.3 `it_should_count_a_remembered_discarded_upload_toward_the_limit` replaces `it_should_not_count_a_remembered_discarded_upload_toward_the_limit`: remembered ids no longer widen the listing or the window. Update the `within_limit`, `is_skipped` and `sync_membership_with_youtube` docs.
- [x] 2.4 `it_should_restore_a_stored_discarded_channel_video`: `restore_discarded` takes the lock, puts stored Discarded videos back to Pending (`Download::restore`, `Video::restore`) and runs the pass.
- [x] 2.5 `it_should_re_add_a_remembered_channel_video_still_among_its_uploads`: `delete_all` on the channel's remembered videos before the pass.
- [x] 2.6 `it_should_forget_a_remembered_channel_video_past_the_limit`.
- [x] 2.7 `it_should_restore_discarded_channel_videos_when_listing_fails`.
- [x] 2.8 `it_should_fail_to_restore_the_discarded_videos_of_an_unknown_channel`: `ChannelNotFound` → 404.
- [x] 2.9 `it_should_restore_a_stored_discarded_playlist_video`.
- [x] 2.10 `it_should_re_add_a_remembered_video_back_in_the_playlist`.
- [x] 2.11 `it_should_fail_to_restore_the_discarded_videos_of_an_unknown_playlist`: `PlaylistNotFound` → 404.
- [x] 2.12 Vitest `VideoDetail` "says a discarded video can be restored": the button and dialog become "Discard", the copy mentions restoring, and existing "Don't download" tests are renamed.
- [x] 2.13 Vitest `VideoListPane` "discards a row's video from its menu after confirming": "Discard…" item plus `DiscardVideoDialog` in `VideoActionsMenu`; `VideoListPane` passes `discard` when `canDiscard`.
- [ ] 2.14 Vitest `VideoListPane` "offers no Discard… for a downloading row".
- [ ] 2.15 Vitest `VideoGrid` "discards a card's video after warning its files will be deleted": `VideoGrid` passes `discard={{ downloaded: true }}`.
- [ ] 2.16 Vitest `VideoGrid` "sends nothing when the discard is cancelled".
- [ ] 2.17 Vitest `TasksView` "discards a failing download after confirming": `DiscardButton` on a `download_video` `AttentionCard`; `useDiscardVideo` also invalidates tasks.
- [ ] 2.18 Vitest `TasksView` "says the discard failed".
- [ ] 2.19 Vitest `TasksView` "offers no Discard on a failing subtitle fetch".
- [ ] 2.20 Vitest `ChannelDetail` "restores the channel's discarded videos after confirming": `DetailHeader` item and `ConfirmDialog`; `ChannelDetail` passes `onRestoreDiscarded`.
- [ ] 2.21 Vitest `PlaylistDetail` "restores the playlist's discarded videos after confirming".
- [ ] 2.22 Vitest `PlaylistDetail` "sends nothing when the restore is cancelled".
- [ ] 2.23 Vitest `ChannelDetail` "tells the user when restoring fails".
- [ ] 2.24 Vitest `CategoryDetail` "offers no Restore discarded videos".

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 None: the change adds or changes no adapter (see design.md Test Plan §2). Mark done once confirmed during implementation.

## 4. Verification

- [ ] 4.1 `scripts/run-tests.sh --lint --web --rust`.
- [ ] 4.2 Update `e2e-tests/tests/discardVideo.spec.js` for the "Discard" label and confirmation copy, then run `scripts/run-tests.sh --e2e`.
- [ ] 4.3 Manual (agent). Start `scripts/run-local.sh --port <free port> --fresh-videos`, seed a channel via the HTTP API, then use the Playwright MCP browser to check:
  - Discarding a row from its "⋮" menu removes it from the list.
  - Discarding a home card removes it from home.
  - A failing download's Needs attention card offers Discard and the card leaves once confirmed.
  - "Restore discarded videos" in the channel's header menu brings the discarded video back as pending. Confirm with the `POST …/discarded/restore` network request and the DOM.
  - With `video_limit` 3 and one video discarded, the channel lists 2 after a Sync.

  Take a screenshot of each, then stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.4 Manual (human). On a phone:
  - The "Discard…" menu item and its confirmation dialog are usable, with the menu's 44 px hit area.
  - The wording of the discard and restore confirmations reads well.
