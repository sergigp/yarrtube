## 1. Backend: serve video files statically

- [x] 1.1 Add `tower-http` (`fs` feature) to `Cargo.toml` and verify `cargo build` succeeds
- [x] 1.2 In `serve.rs`, nest a `ServeDir` rooted at the existing `videos_path` config value at `/media`, registered before the SPA's catch-all fallback
- [x] 1.3 Add a test requesting a known file under the mounted root and verify it returns HTTP 200 with the file's bytes
- [x] 1.4 Add a test requesting a byte range of a known file (`Range` header) and verify it returns HTTP 206 with only that range
- [x] 1.5 Add a test requesting a path that attempts to traverse outside the mounted root and verify no file outside the root is returned
- [x] 1.6 Add a test verifying the existing SPA root (`GET /`) and a known asset still resolve correctly with `/media` mounted alongside it

## 2. Frontend: media URL and video detail view

- [x] 2.1 Add a helper in `web/src/api.js` that builds a video's `/media/...` URL from a playlist's `path` and a video's `filename`
- [x] 2.2 In `PlaylistDetail.jsx`, add selected-video state so clicking a video in the list opens a detail panel
- [x] 2.3 Render the video's full metadata in the detail panel (title, status, quality, filename, playlist path, created/updated timestamps) and verify each field displays correctly in the browser
- [x] 2.4 For videos with status `DOWNLOADED`, render an inline `<video controls>` element pointed at the built media URL; for other statuses, show no player
- [x] 2.5 Manually verify in a browser: select a downloaded video, confirm it plays, and confirm seeking/scrubbing works (exercises the range-request support from 1.4)

## 3. Verification

- [x] 3.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`, and verify all pass
