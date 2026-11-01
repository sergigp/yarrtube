# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures) wired end-to-end: migration `0020_video_chapters.sql`; `VideoChapter`; `Video::chapters` (column added, not yet read/written — persistence is driven by 2.1), `with_chapters`, `is_chapters_fetchable` (returns `false`); `VideoSyncStep::FetchChapters`; `Task::FetchChapters` + payload decode; `ChapterFetcher` (no-op `fetch`); `VideoDownloaderRepository::fetch_chapters` (real impl delegating to `ytdlp::fetch_chapters` returning `Ok(vec![])`, fake with a configurable result + call log); `VideoRepository::update_chapters` (`Ok(())`); `DownloadedVideo::chapters` (always `None`); `FetchChaptersTask` registered as `fetch_chapters` in `application/tasks/mod.rs` and built in `serve.rs`; `ChapterResponse` + `chapters: []` on `VideoResponse`/`CategoryVideoResponse`; web `Chapter` type, `chapters` on `Video`/`CategoryVideo`, `aVideo`/`aCategoryVideo` default `chapters: []`. Done when `cargo build` succeeds, `cargo test --locked` and `npm run check` (in `web/`) pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_record_the_chapters_of_a_downloaded_video` (download_video_task) — a successful download stores the chapters yt-dlp reported.
- [x] 2.2 `it_should_record_no_chapters_for_a_downloaded_video_without_them` (download_video_task) — `Some(vec![])` is stored as checked-with-none.
- [x] 2.3 `it_should_leave_chapters_unchecked_if_unreadable_on_download` (download_video_task) — `None` keeps chapters unchecked; video still Downloaded.
- [x] 2.4 `it_should_schedule_the_chapters_fetch_of_a_downloaded_video` (sync_video_task) — Downloaded + unchecked chapters schedules `fetch_chapters`.
- [x] 2.5 `it_should_not_schedule_a_chapters_fetch_if_already_checked` (sync_video_task) — checked chapters schedule nothing.
- [x] 2.6 A not-yet-downloaded video gets no chapters fetch — already covered by the existing `it_should_schedule_the_download_and_thumbnail_of_a_new_channel_video` (Pending video, chapters unchecked, only thumbnail + download scheduled); no duplicate test added.
- [x] 2.7 `it_should_record_the_fetched_chapters` (fetch_chapters_task) — fetched chapters are stored, nothing else changes.
- [x] 2.8 `it_should_record_no_chapters_if_the_video_has_none` (fetch_chapters_task) — `[]` stored as checked-with-none.
- [x] 2.9 `it_should_leave_chapters_unchecked_if_the_fetch_fails` (fetch_chapters_task) — fetch error: task succeeds, video unchanged.
- [x] 2.10 `it_should_skip_if_chapters_already_checked` (fetch_chapters_task) — no yt-dlp call when chapters already checked.
- [x] 2.11 `it_should_skip_if_the_video_is_gone` (fetch_chapters_task) — missing record: `Ok`, no yt-dlp call.
- [x] 2.12 `it_should_keep_a_playback_position_recorded_during_the_fetch` (fetch_chapters_task) — recording chapters writes only chapters (drives `update_chapters`).
- [x] 2.13 `it_should_include_the_chapters_when_listing_playlist_videos` (http/videos) — listed video carries its chapters in order.
- [x] 2.14 `it_should_list_unchecked_chapters_as_empty` (http/videos) — unchecked chapters listed as `[]`.
- [x] 2.15 `it_should_include_the_chapters_when_listing_category_videos` (http/videos) — category entry carries the chapters.
- [x] 2.16 `it_should_run_fetch_chapters_in_the_subtitles_lane_exclusive_per_video` (domain/task) — lane `Subtitles`, exclusivity key `video:<id>`.
- [x] 2.17 `it_should_list_a_fetch_chapters_task_with_video_and_playlist_names` (http/tasks) — the task view resolves the video title and owner name.
- [x] 2.18 `describes chapter fetches` (web `lib/tasks.test.ts`) — "Fetching chapters of <video> in <source>", placeholder when unresolved, Downloads tab, download icon; `npm run test` passes.

## 3. Infrastructure adapters (TDD)

`infrastructure/shared/ytdlp.rs` (fake `yt-dlp` script)
- [x] 3.1 `it_should_parse_the_printed_chapters_of_a_download` — chapters JSON line parsed; timestamp/date/duration/filename still parsed.
- [x] 3.2 `it_should_report_no_chapters_when_yt_dlp_prints_na` — `NA` → `Some(vec![])`.
- [x] 3.3 `it_should_report_unreadable_chapters_as_unchecked` — unparseable line → `None`, download still succeeds.
- [x] 3.4 `it_should_pass_the_embed_chapters_flag_to_yt_dlp` — `--embed-chapters` in the download args.
- [x] 3.5 `it_should_fetch_the_chapters_without_downloading` — `--skip-download`, printed line parsed, fractional seconds floored.
- [x] 3.6 `it_should_fail_the_chapters_fetch_on_a_clean_failed_exit` — non-zero exit → `Err`.

`infrastructure/repositories/sqlite_video_repository.rs`
- [x] 3.7 `it_should_round_trip_chapters_none_empty_and_some` — `None`, `Some([])`, `Some([..])` persist distinctly.
- [x] 3.8 `it_should_update_only_the_chapters` — `update_chapters` leaves every other column as stored.

`infrastructure/repositories/youtube_video_downloader_repository.rs` (contract, `#[ignore]`)
- [x] 3.9 `it_should_fetch_the_chapters_of_a_real_video` — `O728EFaXuBk` (Blender Studio, CC BY, 299 s) → its 9 chapters; verify with `scripts/run-tests.sh --only contract` while iterating.
- [x] 3.10 `it_should_download_a_video_with_its_chapters_embedded` (contract) — a real 480p download of `O728EFaXuBk` reports its 9 chapters and the mp4 carries 9 matching chapter markers (`ffprobe`).

## 4. Verification

- [x] 4.1 Mention embedded chapters in `doc/PLEX.md` (new downloads only; Plex chapter thumbnails setting) and verify the doc reads correctly.
- [x] 4.2 `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` and `scripts/run-tests.sh` (no flags, every suite incl. contract and e2e) all pass.
- [x] 4.3 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, track a playlist or channel containing `NklJAUizpZs` via the HTTP API, wait for the download, then assert (a) its listing entry has the 3 chapters, (b) `ffprobe -show_chapters` on the downloaded file lists 3 chapters, (c) after setting its `chapters` back to NULL and reconciling, a `fetch_chapters` task runs and restores them. Stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.4 Manual (human): in Plex (and any other client in use), open the downloaded episode and check the chapter menu shows Game 1–3 and jumping works; optionally enable chapter thumbnails.
