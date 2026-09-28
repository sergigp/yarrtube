## 1. Walking skeleton

- [x] 1.1 Create everything listed in design.md's Files and Types & Signatures:
  - Add `VideoView` and export it from `domain::video`.
  - Add the `video_metadata_repository` parameter to `VideoSearcher::new`, wire it in `src/serve.rs` and pass `SqliteVideoMetadataRepository` in the existing handler tests.
  - Change `list` / `list_for_channel` to return `Vec<VideoView>`. `view` returns `VideoView { video, metadata: None }` for now.
  - Add the three nullable fields to `VideoResponse` with `From<VideoView>`, mapping all three to `None` for now.
  - Done when `cargo build` succeeds and all existing tests pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_include_the_metadata_when_listing_playlist_videos`: listing playlist videos returns the publish time, description and channel name from the video's metadata.
- [ ] 2.2 `it_should_include_the_metadata_when_listing_channel_videos`: listing channel videos returns the same metadata fields.
- [ ] 2.3 `it_should_report_absent_metadata_when_listing_a_video_without_it`: a video with no metadata row is listed with all three fields absent.

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 None. The change reuses `SqliteVideoMetadataRepository::find`, which is already tested. Mark this done without changes.

## 4. Verification

- [ ] 4.1 Update `VideoDetail.jsx` to match the web-ui spec delta:
  - The meta line is always visible. The channel name appears only without the `channel` prop. Show "Published <date>" and "Synced <relative>", with the full date and time on hover.
  - The status badge shows only when not `DOWNLOADED`, and the quality badge is removed.
  - The description is clamped to 4 lines with a "Show more" that appears only on overflow. Keep line breaks and link URLs with `target="_blank" rel="noopener"`.
  - Verify with `npm run build` and `npm run lint` in `web/`.
- [ ] 4.2 Update `waitForVideoStatus` in `smoke-tests/helpers/video.js`: for `DOWNLOADED`, wait for the meta line's "Synced" text instead of a "Downloaded" badge. Verify with `scripts/run-smoke-tests.sh`, where all specs pass.
- [ ] 4.3 Run `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings`. All must pass.
- [ ] 4.4 Manual check with `scripts/run-local.sh`, at desktop width and at a mobile width of about 375px:
  - A playlist view shows the channel name; a channel view doesn't.
  - The collapsed mobile pane still shows the meta line.
  - A long description clamps and expands.
  - Links in the description open in a new tab.
  - A pending video shows its status badge and no meta line.
