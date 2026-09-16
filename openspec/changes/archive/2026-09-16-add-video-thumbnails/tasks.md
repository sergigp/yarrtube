## 1. yt-dlp invocation

- [x] 1.1 Add `--embed-thumbnail --write-thumbnail --convert-thumbnails jpg` to the args built in `ytdlp::download_video` (`src/infrastructure/shared/ytdlp.rs`), alongside the existing quality/remux flags, and verify via a `FakeYtDlp`-based test asserting `captured_args()` contains the new flags.
- [x] 1.2 Add a helper that computes the expected thumbnail filename (`{video_filename_stem}.jpg`) given a saved video filename, and a way to check whether that file exists in a given output directory (reusing `VideoFileRepository` rather than raw `std::fs` where it's invoked from domain code) — verify with unit tests covering: stem with the `[<video_id>]` collision suffix, and a filename with no extension edge case.

## 2. Domain model

- [x] 2.1 Add `thumbnail_filename: Option<String>` to `Video` (`src/domain/video/video.rs`), set by `mark_downloaded` (new parameter) and cleared by `reset_for_redownload` — verify by updating/extending `video.rs`'s existing unit tests (`it_should_transition_to_downloaded_when_marked_downloaded`, `it_should_reset_a_downloaded_video_back_to_pending_for_redownload`) to assert the new field.
- [x] 2.2 Add `thumbnail_filename` as a nullable column to the `videos` table (`src/infrastructure/repositories/sqlite_video_repository.rs`) and thread it through `save`/`find`/`update` — verify with a round-trip test (save a video with a thumbnail filename, find it back, assert equality) alongside the existing repository tests.

## 3. Download flow wiring

- [x] 3.1 Inject `VideoFileRepository` into `VideoDownloader` (`src/domain/services/video_downloader.rs`) and, on a successful download, derive and check for the thumbnail file before calling `mark_downloaded` with it (or `None` if absent) — verify with a test where the fake downloader "creates" a matching thumbnail file and `VideoDownloader::download` records it, and a second test where no thumbnail file is present and `thumbnail_filename` stays `None`.
- [x] 3.2 Verify end-to-end (a test invoking a real `FakeYtDlp` script that writes both a fake video file and a fake `.jpg` sibling) that `download_video` completing successfully is compatible with the thumbnail-detection step in 3.1.

## 4. Reconciliation

- [x] 4.1 Update `video_reconciler.rs`'s orphan-detection set to include each `Downloaded` video's `thumbnail_filename` alongside `filename` — verify with a test asserting a thumbnail file matching a downloaded video's recorded `thumbnail_filename` is NOT deleted on reconcile, and one asserting an unrecorded stray `.jpg` still IS deleted.
- [x] 4.2 Apply the same change to `channel_video_reconciler.rs` (mirrors `video_reconciler.rs`) with equivalent tests.
- [x] 4.3 Update `reset_for_redownload` call sites in both reconcilers so a video reset for redownload clears its `thumbnail_filename` too (covered by the domain-level test in 2.1; add a reconciler-level test only if the reset path isn't already exercised).

## 5. Cleanup

- [x] 5.1 Update `VideoFileDeleter::delete_video_file` (`src/domain/services/video_file_deleter.rs`) to also delete a video's `thumbnail_filename` file, if recorded, alongside its main file — verify with tests: thumbnail present and deleted, thumbnail absent no-ops without error, video has no recorded thumbnail filename (no-op for the thumbnail half).
- [x] 5.2 Update call sites that invoke `delete_video_file` (the removal subscribers) to pass the video's `thumbnail_filename` through — verify by checking existing subscriber tests still pass and, where they assert deletion calls, extend them to cover the thumbnail argument.

## 6. HTTP surface

- [x] 6.1 Add `thumbnail_filename` to `VideoResponse` (`src/application/http/videos/dto.rs`) — verify with a test asserting the field serializes correctly for both `Some` and `None`.

## 7. Full verification

- [x] 7.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked` and confirm all pass.
- [x] 7.2 Manually run a real download (`./target/release/yarrtube download <playlist_id> <output_path>` or via `serve`) against a real playlist and confirm on disk: the `.mp4` has embedded cover art (e.g. via `ffprobe`), a matching `.jpg` sibling exists, and a subsequent reconcile pass does not delete it.
