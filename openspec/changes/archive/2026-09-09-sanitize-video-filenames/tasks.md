## 1. Domain: title sanitization

- [x] 1.1 Add a sanitization function/value object in `domain/video/` that turns a raw title into a safe base filename (no extension, no video ID) — collapsing/trimming whitespace, replacing the filesystem-unsafe characters `/ \ : * ? " < > |` with a safe separator, stripping decorative/emoji symbols by Unicode category while preserving letters/digits/accents, and truncating to 150 bytes on a UTF-8 boundary — and verify with unit tests covering each rule individually and combined (mirroring `PlaylistName`'s test style).
- [x] 1.2 Add a regression test using titles drawn from the real messy examples (e.g. `"9 AI Concepts Explained in 7 minutes: AI Agents, RAGs, Tokenization, RLHF..."`, `"＂Got any hobbies？＂"`, `"New Skills! v1.2 brings ⧸wait-what..."`) and verify the sanitized output is whitespace-clean, free of reserved/decorative characters, and preserves acronyms and accented words.

## 2. Infrastructure: yt-dlp invocation and collision handling

- [x] 2.1 Change `infrastructure/shared/ytdlp.rs::download_video` to accept a desired base filename, list the output directory for an existing file whose stem matches it, and pass either `-o "<base>.%(ext)s"` or, on collision, `-o "<base> [<video_id>].%(ext)s"` to `yt-dlp` — verify with unit tests using the existing `FakeYtDlpOnPath` test support, asserting the constructed command/args for both the no-collision and collision cases.
- [x] 2.2 Update `VideoDownloaderRepository::download` (and its fake) in `infrastructure/repositories/youtube_video_downloader_repository.rs` to take and forward the desired filename, and verify existing tests still pass with the new parameter threaded through.

## 3. Wire up call sites

- [x] 3.1 Update `VideoService::download_video` (`domain/video/service.rs`) to compute the sanitized filename from `video.title` before calling the downloader port, and verify with a service-level test (using the fake downloader) asserting the filename passed down matches the sanitized title, not the raw one.
- [x] 3.2 Update `YtDlpDownloaderClient::download_all` (`infrastructure/client/youtube_downloader_client.rs`) to compute the sanitized filename per video the same way before downloading, and verify with a test/manual check that the CLI path produces the same filenames as the daemon path for the same title.

## 4. Verification

- [x] 4.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`, and verify all pass.
- [ ] 4.2 Manually run `./target/release/yarrtube download <playlist_id> <output_path>` against a small real playlist containing at least one messy/clickbait title, and verify the resulting filenames on disk are clean and contain no `[VIDEO_ID]` suffix (absent a collision).
