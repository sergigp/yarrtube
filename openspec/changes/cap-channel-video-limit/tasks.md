Follow `/rust-architect` conventions for every Rust task (value object rules are tested exhaustively in the value object; handler tests keep one invalid-value test per endpoint), and end each task with a commit.

## 1. Value Object

- [x] 1.1 In `src/domain/channel/video_limit.rs`, add `const MAX_VIDEO_LIMIT: i64 = 1000`, make `VideoLimit::new` reject any value outside `1..=MAX_VIDEO_LIMIT` with `ValidationError("Video limit must be between 1 and 1000 (got N)")`, and replace the `value as u32` cast with `u32::try_from`; verify `cargo build` succeeds and `grep -n " as u32" src/domain/channel/video_limit.rs` finds nothing.
- [ ] 1.2 Update the `video_limit.rs` tests to assert exact results: `Ok(VideoLimit(1))` for 1 and `Ok(VideoLimit(1000))` for 1000, and `Err(ValidationError("Video limit must be between 1 and 1000 (got N)"))` for 0, -5, 1001, 4294967296 and 4294967306. The last two are the wrapping values from the bug. Verify `cargo test video_limit` passes.

## 2. Persistence Boundary

- [ ] 2.1 Add `it_should_round_trip_the_maximum_video_limit` to `src/infrastructure/repositories/sqlite_channel_repository.rs`: insert a channel with `VideoLimit::new(1000)` into the in-memory SQLite repository and assert `find` returns the whole channel unchanged. Verify the test passes.

## 3. HTTP

- [ ] 3.1 Confirm `src/application/http/channels/mod.rs` needs no handler change: the `?` on `VideoLimit::new` already maps `ValidationError` to 400. Verify `cargo test http::channels` passes unchanged. Add no handler test for the range, since the value object covers it.

## 4. Web UI

- [ ] 4.1 Add `max="1000"` to the video limit `<Input>` in `web/src/components/AddDialog.jsx`, next to `min="1"`. Verify `npm run build` and `npm run lint` succeed in `web/`, and check in the dev server that entering 1001 marks the field invalid and blocks submission.

## 5. Verification

- [ ] 5.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` and `cargo test --locked`; verify all three succeed.
- [ ] 5.2 With `scripts/run-local.sh` running, `POST /channels` with `video_limit: 4294967306`. Verify the response is a 400 with body `{"error": "Video limit must be between 1 and 1000 (got 4294967306)"}`, and that `GET /channels` still returns 200 with no new channel.
