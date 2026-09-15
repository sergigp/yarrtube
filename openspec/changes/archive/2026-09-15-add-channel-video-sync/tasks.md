## 1. Domain model: decouple Video from Playlist

- [x] 1.1 Rewrite `domain/video/video.rs`: drop `playlist_id`, add a surrogate id (own value object, e.g. `VideoRecordId`) and rename today's `video_id: VideoId` field to something like `youtube_id: VideoId` (keep the `VideoId` value object itself unchanged — it still wraps a YouTube video id). Keep `title`, `status`, `quality`, `filename`, `created_at`, `updated_at`. Drop `position`. Verify: `cargo test domain::video` passes with updated unit tests for `create`/`start_download`/`mark_downloaded`/`mark_errored*`/`reset_for_redownload`.
- [x] 1.2 Add `domain/playlist_video/` (or equivalent placement per `rust-architect`): `PlaylistVideo` entity — own id, `playlist_id`, `video_id` (FK to the new `Video` surrogate id), `position`, `created_at`. Verify: unit tests for construction.
- [x] 1.3 Add `domain/channel_video/`: `ChannelVideo` entity mirroring `PlaylistVideo` with `channel_id` in place of `playlist_id`. Verify: unit tests for construction.
- [x] 1.4 Add `path: PlaylistPath` (or an equivalent value object) to `domain/channel/channel.rs::Channel`, threaded through `Channel::create`. Verify: `channel.rs` unit test asserts the stored path.

## 2. Repositories and schema

- [x] 2.1 Rewrite `SqliteVideoRepository`: `CREATE TABLE videos` reflects the final `Video` shape directly (surrogate id as primary key, `youtube_id` no longer unique), no `ALTER TABLE`. CRUD keyed by the surrogate id only — no `playlist_id` anywhere in this repository. Update `FakeVideoRepository` to match. Verify: `cargo test sqlite_video_repository` passes.
- [x] 2.2 Add `SqlitePlaylistVideoRepository` (`infrastructure/repositories/sqlite_playlist_video_repository.rs`): `CREATE TABLE playlist_videos` (own id, `playlist_id`, `video_id`, `position`, `created_at`); methods to save/find-by-`(playlist_id, youtube video id)`/list-for-playlist/delete/delete-all-for-playlist. Add a `FakePlaylistVideoRepository` for tests. Verify: repository unit tests, including ordering by `position` (mirrors the removed ordering test from `sqlite_video_repository.rs`).
- [x] 2.3 Add `SqliteChannelVideoRepository` mirroring 2.2 for `channel_videos`, keyed by `channel_id`. Add a `FakeChannelVideoRepository`. Verify: repository unit tests.
- [x] 2.4 Add `channels` table's `path TEXT NOT NULL` column directly in its `CREATE TABLE` (no `ALTER TABLE`). Update `SqliteChannelRepository`/`FakeChannelRepository` CRUD to round-trip it. Verify: repository unit tests assert `path` round-trips.
- [x] 2.5 Grep the codebase for `ALTER TABLE` and confirm zero matches after 2.1-2.4 (the one existing instance, the `videos.position` bolt-on, is removed by the `videos` table rewrite). Verify: `grep -rn "ALTER TABLE" src/` returns nothing.

## 3. Channel video discovery (yt-dlp)

- [x] 3.1 Add a `ChannelVideosRepository` port (trait) with a method to list a channel's current videos capped at a given limit, returning `{ youtube_id, title, position }` per video (position = recency rank, 0 = newest).
- [x] 3.2 Implement it via `yt-dlp --flat-playlist --print-json -I 1:<limit> <channel videos URL>`, parsing one JSON object per stdout line (fields `id`, `title`) — no delimited/`--print "%(title)s | ..."` parsing. Handle a clean non-zero exit / empty output as "no videos" (`Ok(vec![])`), not an error; handle a missing `yt-dlp` binary and unparseable output as `Err`, mirroring `infrastructure/shared/ytdlp.rs::download_video`'s error posture.
- [x] 3.3 Add a `FakeChannelVideosRepository` for tests.
- [x] 3.4 Unit test: a discovered video whose title contains a `|` character is parsed with its exact title intact, and does not corrupt the parsing of adjacent videos in the same `yt-dlp` output (this is the scenario the delimited `--print` format could not have satisfied — write it as a regression test against JSON-line parsing). Verify: `cargo test` covers this exact case, asserting on the parsed `title` field verbatim.
- [x] 3.5 Unit test: a title containing a literal newline or other unusual whitespace is still parsed correctly as a single JSON object per line. Verify: dedicated test case.

## 4. Domain events

- [x] 4.1 Replace `DomainEvent::VideoAdded`/`VideoDeleted` with `VideoAddedToPlaylist`/`VideoRemovedFromPlaylist` (same fields as today's `VideoAdded`/`VideoDeleted`) in `domain/event/domain_event.rs`. Verify: existing `event_type()`/`payload()` unit tests updated and passing.
- [x] 4.2 Add `DomainEvent::VideoAddedToChannel { channel_id, video_id }` and `VideoRemovedFromChannel { channel_id, video_id, title, filename, was_downloaded }`. Verify: unit tests for `event_type()`/`payload()`.

## 5. Domain services: orchestration

- [x] 5.1 Rework the playlist-membership-diff logic (today's `VideoReconciler::sync_playlist_membership`) to orchestrate across `VideoRepository` + `PlaylistVideoRepository` explicitly: create a `Video` row then its owning `PlaylistVideo` row for a new member; delete the `PlaylistVideo` row then its `Video` row for a removed member; publish `VideoAddedToPlaylist`/`VideoRemovedFromPlaylist`. Verify: existing `VideoReconciler` unit tests updated and passing.
- [x] 5.2 Add the channel equivalent (new domain service, e.g. `ChannelVideoReconciler`): fetches current videos via `ChannelVideosRepository` capped at `Channel.video_limit`, diffs against stored `ChannelVideo` rows the same way, orchestrating across `VideoRepository` + `ChannelVideoRepository`, publishing `VideoAddedToChannel`/`VideoRemovedFromChannel`. Include `reconcile`/`force_reconcile` methods mirroring `VideoReconciler`'s (recurring reschedule vs. on-demand, both no-op for a since-deleted channel). Verify: unit tests mirroring `video_reconciler.rs`'s, including: video within top-N persisted, video that ages out of top-N evicted, no-op for deleted channel, yt-dlp failure leaves stored videos untouched.
- [x] 5.3 Update cascading deletes: deleting a `Playlist` deletes its `PlaylistVideo` rows and, for each, its owned `Video` row (not just today's flat `delete_all_for_playlist`). Same for deleting a `Channel` and its `ChannelVideo` rows. Verify: unit tests on the delete services assert both tables are empty afterward.
- [x] 5.4 Rewrite `VideoDownloader::download` to drop its `PlaylistRepository` dependency entirely: signature becomes `(video's surrogate id, quality, output_dir)`, looks up/updates only via `VideoRepository`. Verify: existing `VideoDownloader` unit tests updated (no playlist fixture needed) and passing.
- [x] 5.5 Rewrite `VideoFileDeleter::delete_video_file` to drop its `PlaylistRepository` dependency: signature becomes `(filename, output_dir)`. Keep `delete_playlist_video_files`/add a channel equivalent for whole-directory deletion (still takes the pre-resolved `path`, unaffected by this refactor beyond already not depending on live playlist state). Verify: existing `VideoFileDeleter` unit tests updated and passing.
- [x] 5.6 Update `CustomPlaylistVideoAdder`/`CustomPlaylistVideoRemover` to orchestrate through `VideoRepository` + `PlaylistVideoRepository` and publish `VideoAddedToPlaylist`/`VideoRemovedFromPlaylist`. Verify: existing unit tests updated and passing.

## 6. Tasks

- [x] 6.1 Update `Task::DownloadVideo` to `{ video_id, quality, output_dir }` (drop `playlist_id`) and `Task::DeleteVideoFile` to `{ filename, output_dir }` (drop `playlist_id`, `video_id`, `title` if no longer needed for logging — keep whichever fields `VideoFileDeleter` still needs). Update `task_type()`/`payload()`/decode helpers. Verify: existing task payload round-trip unit tests updated and passing.
- [x] 6.2 Add `Task::ReconcileChannel { channel_id }` mirroring `Task::ReconcilePlaylist`. Verify: payload round-trip unit test.
- [x] 6.3 Update `application/tasks/download_video_task.rs` and `delete_video_file_task.rs` handlers for the new agnostic payloads. Verify: existing handler unit tests updated and passing.
- [x] 6.4 Add `application/tasks/reconcile_channel_task.rs` mirroring `reconcile_playlist_task.rs`, and register it (and `download_video`/`delete_video_file`'s unchanged registration) in `application/tasks/mod.rs::registry`. Verify: handler unit test; `registry()` includes `"reconcile_channel"`.

## 7. Subscribers

- [x] 7.1 Rename `DownloadVideoOnVideoAdded` (or add a sibling) to react to `video_added_to_playlist`, resolving `quality`/`output_dir` from `PlaylistRepository` and scheduling the now-agnostic `Task::DownloadVideo`. Verify: existing unit tests updated and passing.
- [x] 7.2 Add `DownloadVideoOnVideoAddedToChannel`, mirroring 7.1 against `ChannelRepository`. Verify: unit tests mirroring 7.1's, including "channel no longer exists" no-op.
- [x] 7.3 Rename/split `DeleteVideoFileOnVideoDeleted` into a playlist-reacting subscriber (`video_removed_from_playlist`) that resolves `output_dir` from `PlaylistRepository` before scheduling the agnostic `Task::DeleteVideoFile`. Verify: existing unit tests updated and passing.
- [x] 7.4 Add `DeleteVideoFileOnVideoRemovedFromChannel` mirroring 7.3 against `ChannelRepository`. Verify: unit tests mirroring 7.3's.
- [x] 7.5 Add `ReconcileOnChannelCreated` mirroring `ReconcileOnPlaylistCreated`, reacting to `channel_created` and calling the new channel reconcile service. Verify: unit test mirroring `reconcile_on_playlist_created.rs`'s.
- [x] 7.6 Register all of the above in `application/subscribers/mod.rs::registry` under their event type keys (`video_added_to_playlist`, `video_added_to_channel`, `video_removed_from_playlist`, `video_removed_from_channel`, `channel_created`). Verify: registry unit/integration coverage, or a compile-time check that every new event type has at least one subscriber. (Also added `channel_deleted` → `DeleteChannelFilesOnChannelDeleted`, mirroring `playlist_deleted`, needed for video-cleanup's "channel directory deleted when channel is deleted" requirement — added `path` to `ChannelDeleted` and a new `Task::DeleteChannelFiles`/handler to support it.)

## 8. HTTP

- [x] 8.1 Update `create_channel` (`application/http/channels/mod.rs`, `dto.rs`) to require and persist `path`, mirroring `create_playlist`'s path validation/error responses. Verify: existing channel HTTP tests updated; new tests for missing/invalid path (empty, absolute, `..` segment, empty segment) mirroring `playlists/mod.rs`'s equivalents.
- [x] 8.2 Add `POST /channels/{handle}/reconcile`, calling the channel reconcile service's `force_reconcile`, mirroring `reconcile_playlist`. Verify: HTTP tests mirroring the playlist reconcile endpoint's (existing channel reconciled, repeated calls don't change pending task count, nonexistent channel is a no-op 204, invalid handle is 400).
- [x] 8.3 Add `GET /channels/{handle}/videos`, mirroring `list_videos` for playlists but ordered by recency. Verify: HTTP tests mirroring `video-listing`'s (channel has videos / no videos / doesn't exist).
- [x] 8.4 Update video playback route resolution to serve a channel-owned video's file from its channel's `path`, alongside the existing playlist case. Verify: existing playback HTTP tests updated; new test for a channel-owned video's media URL. (The `/media` mount was already container-agnostic — no production change needed; added a test proving a channel-style nested path serves correctly.)

## 9. Composition root

- [x] 9.1 Wire the new repositories, `ChannelVideoReconciler`, `ChannelVideosRepository` (yt-dlp-backed), new subscribers, and the new task handler in `serve.rs::build_application()` and `AppState`. Verify: `cargo build --release` succeeds; `serve` starts against a fresh (empty) database without error.

## 10. Full-suite verification

- [x] 10.1 `cargo fmt --all -- --check` passes.
- [x] 10.2 `cargo clippy --all-targets --all-features --locked -- -D warnings` passes.
- [x] 10.3 `cargo test --locked` passes, including the new `|`-in-title regression test (3.4) and the top-N eviction test (5.2). (403/403 passing.)
- [ ] 10.4 Manual smoke test against a real channel (e.g. the `@ThePrimeagen` example from exploration): create a channel with a small `video_limit`, confirm it downloads that many most-recent videos and that a subsequent reconcile evicts one once a newer upload pushes it past the limit. **Partially done manually** — the user created a real channel (`@mostly_mac`, `video_limit: 3`) against a live YouTube API key/yt-dlp and confirmed it downloaded exactly 3 most-recent videos, and separately confirmed live that filesystem healing (missing-file redownload) works end-to-end. The eviction-on-new-upload scenario specifically (a newer video pushing an older one past the limit) was not exercised, since no new video was uploaded to the test channel during testing.
- [x] 10.5 Update `README.md` (env vars / setup) and `openspec/project.md` if either documents the current `videos` schema, the `VideoAdded`/`VideoDeleted` event names, or channel creation's request shape. (`openspec/project.md` doesn't exist in this repo. README didn't document the schema/event names, but did have a "coming soon: channel subscriptions" bullet and no channel API docs at all — updated both.)

## Post-implementation fixes (found during manual testing)

- [x] Web SPA: the "Add Channel" form was missing the required `path` field entirely (`CreateChannelForm.jsx`, `api.js`).
- [x] Web SPA: `ChannelList` had no click-through to a channel's videos at all — added `ChannelDetail.jsx` (mirroring `PlaylistDetail.jsx`), wired `onSelect` through `ChannelList`/`App.jsx`, and added a "Reconcile" action to `ChannelActionsMenu`, plus the corresponding `fetchChannelVideos`/`reconcileChannel` API calls.
- [x] `scripts/run-local.sh`: didn't pass the `PATH`-resolved `yt-dlp` location through as `YTDLP_PATH`, so the app always fell back to the Docker-only default `/app/bin/yt-dlp`.
- [x] `src/infrastructure/client/ytdlp_updater.rs`: the self-updater always downloaded the Linux yt-dlp release asset regardless of host OS, which — once `YTDLP_PATH` correctly pointed at a real local binary — would have silently overwritten a working macOS binary with a non-executable one. Now platform-aware (`yt-dlp_macos` on macOS, `yt-dlp_linux` elsewhere).
- [x] `scripts/run-local.sh`: defaulted to a fresh `/tmp` directory for `YARRTUBE_VIDEOS_PATH` on every run while the SQLite DB persisted, orphaning previously-downloaded files after every restart. Now defaults to a persistent `<repo>/videos` directory (gitignored), matching the DB; `--fresh-videos` opts into the old ephemeral behavior.
- [x] `ChannelVideoReconciler` was missing the filesystem-healing pass `VideoReconciler` has for playlists (heal a `Downloaded` video whose file is missing/non-mp4, recover permanently `Errored` videos, delete orphaned files) — added, verified against a live channel (deleted a downloaded file on disk, force-reconciled, confirmed it was detected, reset to `PENDING`, and redownloaded).
