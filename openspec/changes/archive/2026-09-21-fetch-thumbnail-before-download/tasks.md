## 1. yt-dlp thumbnail-only fetch

- [x] 1.1 Add `FetchedThumbnail` struct and `fetch_thumbnail` fn to `src/infrastructure/shared/ytdlp.rs` (`--skip-download --write-thumbnail --convert-thumbnails jpg`, reusing `resolve_folder_collision`/`ensure_output_dir`/`output_retrying_busy`/`remove_video_dir_best_effort`); verify with unit tests covering: successful fetch returns folder+filename, clean failure (no thumbnail) returns `Ok(None)` and removes the created folder, missing `yt-dlp` binary returns `Err`, folder-collision suffixing matches `download_video`'s
- [x] 1.2 Add `existing_folder: Option<&str>` param to `download_video`; when `Some`, skip `resolve_folder_collision` and use it directly; verify with a unit test asserting the passed folder is used verbatim and no collision check runs against it

## 2. Downloader port

- [x] 2.1 Add `fetch_thumbnail` to `VideoDownloaderRepository` trait and implement on `YtDlpVideoDownloaderRepository`; verify with a unit test mirroring the existing `download` port test
- [x] 2.2 Add `existing_folder: Option<&str>` to `VideoDownloaderRepository::download` and thread it through `YtDlpVideoDownloaderRepository::download`; verify existing port tests still pass with `None` passed explicitly
- [x] 2.3 Update `FakeVideoDownloaderRepository` to record `fetch_thumbnail` calls and support a configurable result, and to accept the new `download` param; verify existing callers compile and pass `None`

## 3. Domain model

- [x] 3.1 Add `Video::with_thumbnail(self, thumbnail_filename, now) -> Self` (only touches `thumbnail_filename`/`updated_at`); verify with a unit test asserting status/filename/quality are untouched

## 4. ThumbnailFetcher service

- [x] 4.1 Create `src/domain/services/thumbnail_fetcher.rs` with `ThumbnailFetcher::fetch(&self, video: &Video, output_dir: &Path)`: no-ops if `video.thumbnail_filename` is already set, otherwise calls the downloader port and persists via `with_thumbnail` on success, logs and swallows on `Ok(None)`/`Err`; verify with unit tests covering: no-op when already set, success persists `"{folder}/{filename}"`, clean failure leaves the video untouched, port error leaves the video untouched

## 5. Real download reuses the fetched folder

- [x] 5.1 In `VideoDownloader::download`, derive `existing_folder` via `top_level_entry(video.thumbnail_filename.as_deref())` and pass it to `video_downloader_repository.download`; verify with a unit test asserting the fake downloader receives the expected folder when `thumbnail_filename` is set, and `None` when it isn't

## 6. Wire thumbnail fetch into the three video-creation sites

- [x] 6.1 Inject `Arc<ThumbnailFetcher>` into `VideoReconciler`; in `sync_playlist_membership`, call `fetch(&video, &output_dir)` for each newly created video, before publishing `VideoAddedToPlaylist`; verify with a unit test asserting the fake downloader was called for a newly added video and the persisted video carries the fetched thumbnail
- [x] 6.2 Inject `Arc<ThumbnailFetcher>` into `ChannelVideoReconciler`; same change in `sync_channel_membership` before publishing `VideoAddedToChannel`; verify with an equivalent unit test
- [x] 6.3 Add a `videos_path: String` field and `Arc<ThumbnailFetcher>` to `CustomPlaylistVideoAdder`; call `fetch(&video, &output_dir)` in `add` before publishing `VideoAdded`; verify with a unit test asserting the fetch runs and a request still succeeds when the fetch fails
- [x] 6.4 Verify (integration-level unit test in each of the three modules) that a thumbnail-fetch failure never prevents the video from being persisted or the event from being published

## 7. Reconcile: orphan-sweep protection and missing-thumbnail recovery

- [x] 7.1 In `VideoReconciler::reconcile_filesystem`, widen `protected_top_level` to include every stored video's `thumbnail_filename` (not just `Downloaded` videos'); verify with a unit test asserting a `Pending` video's pre-fetched thumbnail folder survives a reconcile pass
- [x] 7.2 Add a missing-thumbnail recovery step to `VideoReconciler`'s reconcile pass: for every stored video with no `thumbnail_filename`, call `thumbnail_fetcher.fetch`; verify with a unit test asserting a video with no thumbnail gets one after reconcile, and a video that already has one is not re-fetched
- [x] 7.3 Apply the same two changes (7.1, 7.2) to `ChannelVideoReconciler`; verify with equivalent unit tests

## 8. Application wiring

- [x] 8.1 Construct `ThumbnailFetcher` in `src/serve.rs` (or wherever services are built) and pass it into `VideoReconciler`, `ChannelVideoReconciler`, and `CustomPlaylistVideoAdder`; pass `videos_path` into `CustomPlaylistVideoAdder`; verify the crate builds (`cargo build --release`) and existing integration/HTTP tests for custom-playlist video addition still pass

## 9. Full verification

- [x] 9.1 Run `cargo test --locked` and confirm all tests pass
- [x] 9.2 Run `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` and confirm both are clean
- [x] 9.3 Manually verify via `scripts/run-local.sh`: add a video to a tracked playlist and observe its thumbnail file appear in `videos/` before the video's own download completes
