## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md's Files and Types & Signatures, wired end to end, with today's channel dialog kept. **Rust:**
  - Add `ChannelPreview` and `PreviewChannelError`, exported from `domain/channel`.
  - Add `ChannelPreviewer` (`preview` returns `Err(YoutubeChannelNotFound(id))`) to `domain/services`. Add it to `ApiServices` and `serve.rs`.
  - Add `PreviewChannelQuery`, `ChannelPreviewResponse`, and a `preview_channel` handler that returns the previewer's error mapped to 404/502, routed at `GET /channels/preview`.

  **Frontend** (the channel dialog doesn't call the preview yet):
  - Add `previewChannel` to `api.js`, and `useChannelPreview` with the `channelPreview` key to `queries.js`.
  - Add the `leading` slot to `DestinationNotice`.
  - `channelNoticeLead(videoLimit, title)` accepts the title and ignores it for now.

  Done when:
  - `cargo build` and `cargo test --locked` pass.
  - `npm run build` and `npm run lint` pass in `web/`.
  - `scripts/run-smoke-tests.sh` passes with the existing specs.

## 2. Behaviour (TDD)

Rust cycles: red is an assertion failure, and green is `cargo test --locked` fully passing. Smoke cycles: green is `npm run build`, `npm run lint` and the full `scripts/run-smoke-tests.sh` passing.

- [ ] 2.1 `it_should_preview_a_channel`: the preview returns the channel's handle, title and avatar URL and persists nothing.
- [ ] 2.2 `it_should_preview_a_channel_from_a_youtube_url`: the handle is extracted from a channel URL with trailing segments.
- [ ] 2.3 `it_should_preview_a_channel_without_an_avatar`: a channel YouTube reports no avatar for previews with no avatar.
- [ ] 2.4 `it_should_fail_to_preview_if_invalid_channel_provided`: a handle without `@` is rejected with the value object's message without contacting YouTube.
- [ ] 2.5 `it_should_fail_to_preview_if_channel_missing`: an absent `channel` query parameter is rejected as empty.
- [ ] 2.6 `it_should_fail_to_preview_if_channel_not_found_on_youtube`: a channel YouTube doesn't know is reported as not found.
- [ ] 2.7 `it_should_fail_to_preview_if_youtube_lookup_fails`: a failed lookup is reported as a bad gateway.
- [ ] 2.8 `addChannelDialog.spec.js` › `it should explain a value that is not a channel`: the debounced preview's 400 message shows as an error notice, and submit is disabled while the preview has failed.
  - `channel.spec.js`'s invalid-handle step can no longer submit, so it asserts the error notice and the disabled submit instead.
- [ ] 2.9 `addChannelDialog.spec.js` › `it should state the video limit, title and destination for a handle`:
  - The info notice waits for the lookup: show "Looking up channel…", gate submit on the preview, and render the avatar through `Thumbnail` in the `leading` slot.
  - `channelNoticeLead` names the title for limits 1, many and out of range.
  - `fillChannel` waits for the lookup to settle.
  - The existing `addChannelDialog.spec.js` cases and the channel cases in `addDialogLocation.spec.js` move from `@some-handle` to `SMOKE_CHANNEL_HANDLE`, since a made-up handle now reports "not found".
- [ ] 2.10 `addChannelDialog.spec.js` › `it should report a channel YouTube doesn't know`: a random handle shows "does not exist" as an error notice, and submit stays disabled.
- [ ] 2.11 `channel.spec.js` › lifecycle: re-adding the same handle shows "Already added as" and disables `Create Channel`. The dialog matches the preview's handle against `useChannels()`, ignoring case.

## 3. Infrastructure adapters (TDD)

None: `YoutubeChannelRepository::resolve` already returns the title and avatar URL.

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass.
- [ ] 4.2 `npm run build` and `npm run lint` pass in `web/`, and `scripts/run-smoke-tests.sh` is fully green.
- [ ] 4.3 Manual check at desktop width and at about 375px, with a scratch database and videos directory so local data is untouched:
  - The channel dialog shows only the handle field until something is typed. It then shows "Looking up channel…", followed by the avatar, the limit, the title and the path.
  - A limit or folder edit updates the notice, and `[change]` expands "Advanced options".
  - A handle without `@`, an unknown handle, an already-added channel and a taken folder each show an error notice with submit disabled.
  - A channel without an avatar shows the placeholder.
  - The playlist dialog behaves as before.
