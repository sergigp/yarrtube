## 1. Walking skeleton

- [x] 1.1 Add `ThumbnailFetch` (`Fetched` / `Unavailable { reason }`) and switch `VideoDownloaderRepository::fetch_thumbnail`, `YtDlpVideoDownloaderRepository`, `ytdlp::fetch_thumbnail` and the fake to return it. The existing behaviour maps to `Ok(None)` → `Unavailable { reason: None }`, and the fake's `with_thumbnail_result(Option<FetchedThumbnail>)` keeps its signature. Add `Video::is_thumbnail_fetchable` returning `true`. Update `ThumbnailFetcher::fetch` to match on the enum, keeping the same log line. Done when `cargo build` succeeds and `cargo test --locked` stays green, with no new behaviour.

## 2. Behaviour (TDD)

- [x] 2.1 `fetch_thumbnail_task::it_should_not_fetch_a_thumbnail_for_an_excluded_video`: an Excluded video gets no yt-dlp call and is left unchanged (implement `is_thumbnail_fetchable` for Excluded, and the guard in `ThumbnailFetcher::fetch`). Verify with `cargo test it_should_not_fetch_a_thumbnail_for_an_excluded_video`.
- [x] 2.2 `fetch_thumbnail_task::it_should_not_fetch_a_thumbnail_for_an_errored_video`: an Errored video gets no yt-dlp call and is left unchanged. Verify with `cargo test it_should_not_fetch_a_thumbnail_for_an_errored_video`.
- [x] 2.3 `reconcile_playlist_task::it_should_not_schedule_a_thumbnail_fetch_for_an_excluded_video`: the playlist recovery pass schedules no `FetchThumbnail` for an Excluded video (filter in `schedule_missing`). Verify with `cargo test` on that name.
- [ ] 2.4 `reconcile_playlist_task::it_should_not_schedule_a_thumbnail_fetch_for_an_errored_video_not_due_for_recovery`: an Errored video still inside the recovery cooldown gets no `FetchThumbnail`. Verify with `cargo test` on that name.
- [ ] 2.5 `reconcile_channel_task::it_should_not_schedule_a_thumbnail_fetch_for_an_excluded_video`: channel counterpart of 2.3. Verify with `cargo test` on that name.
- [ ] 2.6 `reconcile_channel_task::it_should_not_schedule_a_thumbnail_fetch_for_an_errored_video_not_due_for_recovery`: channel counterpart of 2.4. Verify with `cargo test` on that name.
- [ ] 2.7 Make the "no thumbnail available" warn in `ThumbnailFetcher::fetch` attach `reason` as a field when `Unavailable { reason: Some(..) }`. Verify by code review against design.md's call stack; logs are not asserted in tests (logging spec).

## 3. Infrastructure adapters (TDD)

`ytdlp.rs`:

- [ ] 3.1 `it_should_return_unavailable_with_the_reason_on_a_clean_failed_thumbnail_fetch`: the fake script writes `ERROR: [youtube] x: Video unavailable` to stderr and exits 1. `fetch_thumbnail` pipes stderr instead of inheriting it, returns `Unavailable { reason: Some(..) }` with warning lines removed, and the folder is removed. This replaces `it_should_return_none_and_remove_the_folder_on_a_clean_failed_exit`. Verify with `cargo test` on that name.
- [ ] 3.2 `it_should_return_unavailable_without_a_reason_when_yt_dlp_prints_na`: adapt the existing NA test to the new type. Verify with `cargo test` on that name.
- [ ] 3.3 `it_should_error_with_the_reason_when_listing_channel_videos_fails`: the fake script writes an error to stderr and exits 1. `list_channel_videos` pipes stderr and returns `Err` whose message contains the reason. This replaces `it_should_return_an_empty_list_on_a_clean_failed_exit`. Verify with `cargo test` on that name. The existing `reconcile_channel_task::it_should_keep_videos_if_listing_fails` keeps covering the reconciler side.

`plex_collection_repository.rs`:

- [x] 3.4 `it_should_add_items_to_a_collection`: PUT `/library/collections/{key}/items` with the members `uri`. Already applied in the working tree and green under `cargo test plex`.
- [x] 3.5 `it_should_remove_an_item_from_a_collection`: DELETE `/library/collections/{key}/items/{ratingKey}`. Already applied in the working tree and green under `cargo test plex`.

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass.
- [ ] 4.2 After deploying, `yarrlogs --since 1h` shows:
  - no Plex `404` on `/library/collections/.../items`, and `added items to Plex collection` for the lagging collections;
  - no raw `ERROR: [youtube] …` lines;
  - no `fetch_thumbnail` tasks for the Excluded Yakari videos.
