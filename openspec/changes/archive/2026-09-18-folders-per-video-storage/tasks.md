## 1. Download mechanics (`src/infrastructure/shared/ytdlp.rs`)

- [x] 1.1 Replace `resolve_collision` (file-stem scan) with a folder-collision check (`output_path.join(desired_filename).exists()` → append ` [{video_id}]`), and add `folder: String` to `DownloadedVideo`
- [x] 1.2 In `download_video`, create the resolved video folder (`ensure_output_dir`), write an empty `meta.nfo` inside it, and run `yt-dlp` with `.current_dir(&video_dir)` and output template `{folder}.%(ext)s`; verify via updated unit tests that: no-collision case creates `<container>/<title>/<title>.mp4` with `meta.nfo` present, collision case suffixes the folder name (not the file), and the returned `DownloadedVideo.filename` stays bare (relative to the video's own folder)
- [x] 1.3 Update/extend the existing `it_should_pass_the_desired_filename_as_the_output_template_when_there_is_no_collision` / `it_should_append_the_video_id_to_the_output_template_on_collision` tests for the new folder-based behavior, and add a test asserting `meta.nfo` is created

## 2. Domain orchestration (`src/domain/services/video_downloader.rs`)

- [x] 2.1 Compose `Video.filename` as `"{downloaded.folder}/{downloaded.filename}"`; run the thumbnail-existence check against `output_dir.join(&downloaded.folder)` instead of `output_dir`, and compose `thumbnail_filename` the same `"{folder}/{name}"` way; verify via `src/application/tasks/download_video_task.rs`'s existing end-to-end test (`it_should_detect_a_real_thumbnail_file_written_by_yt_dlp_alongside_the_video`), updated to assert the new nested `filename`/`thumbnail_filename` shape

## 3. Shared helper for old/new layout interop

- [x] 3.1 Add `src/domain/video/video_output_entry.rs` with `pub fn top_level_entry(relative_path: &str) -> &str` (first path component, or the whole string unchanged if there's no `/`), with unit tests for both the nested and legacy-flat cases, and wire it into `src/domain/video/mod.rs`

## 4. Filesystem repository (`src/infrastructure/repositories/filesystem_video_file_repository.rs`)

- [x] 4.1 Make `VideoFileRepository::delete`'s implementation directory-aware (`symlink_metadata` → `remove_dir_all` for a directory, `remove_file` otherwise), keeping its signature, its "missing is not an error" contract, and existing file-deletion tests passing; add a test deleting a directory
- [x] 4.2 Add `fn file_exists(&self, output_dir: &Path, filename: &str) -> bool` to the trait and `FilesystemVideoFileRepository`, with unit tests for a present nested file, a present flat file, and a missing file; add a matching fake/tracking field to `FakeVideoFileRepository`

## 5. Video deletion (`src/domain/services/video_file_deleter.rs`)

- [x] 5.1 Update `delete_video_file` to call `top_level_entry()` on `filename`/`thumbnail_filename` before deleting, so a new-style video's whole folder is removed and a legacy flat video's two files are removed exactly as before; verify with tests covering both layouts (new-style: folder and its contents gone; legacy: only the two named files gone)

## 6. Reconciliation (`src/domain/services/video_reconciler.rs`, `src/domain/services/channel_video_reconciler.rs`)

- [x] 6.1 Replace the per-video health check (`files.iter().any(|f| f == filename)`) with `video_file_repository.file_exists(&output_dir, filename)` in both reconcilers, keeping the existing `.mp4`-extension check; verify with a test where a downloaded video's file lives in a nested folder and is correctly judged healthy
- [x] 6.2 Replace the flat `protected_filenames` set with `protected_top_level`, built via `top_level_entry()` over every downloaded video's `filename`/`thumbnail_filename`, in both reconcilers; verify with a test proving a healthy new-style video's folder is NOT swept as an orphan, and a genuinely orphaned folder IS removed (via the now directory-aware `delete`)
- [x] 6.3 Run the full existing reconciliation test suites for both reconcilers and confirm all pre-existing (legacy-layout-shaped) assertions still pass unchanged

## 7. Frontend media URL encoding (`web/src/api.js`)

- [x] 7.1 Update `videoMediaUrl` to percent-encode `filename` segment-by-segment (`filename.split('/').map(encodeURIComponent).join('/')`) instead of as one opaque string, mirroring the existing `playlistPath` treatment

## 8. Cross-cutting verification

- [x] 8.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`, and confirm all pass
- [x] 8.2 Manually exercise the app end to end via the `run` skill: download a fresh video into a playlist and confirm on disk it lands as `<container>/<title>/<title>.mp4` + `.jpg` + `meta.nfo`, and that it plays back correctly in the SPA (including its thumbnail); also confirm deleting that video from a custom playlist removes its whole folder
