## 1. Walking skeleton

- [x] 1.1 Add an empty `migrations/0006_playlist_video_position_required.sql` and register it in `sqlite_migrations::apply`. Verify: `cargo build` succeeds and `cargo test --locked` passes.

## 2. Behaviour (TDD)

- [x] 2.1 None new: no new observable behaviour.

## 3. Infrastructure adapters (TDD)

- [x] 3.2 `sqlite_migrations::it_should_reject_a_playlist_video_without_position`: write the table rebuild in 0006. Done before 3.1: against an empty migration only this test can go red. Getting the suite green also gave every test seed a position (`create_with_position(.., 0, ..)`) and deleted the repository's two no-position tests. Verify: the test passes.
- [x] 3.1 `sqlite_migrations::it_should_keep_playlist_videos_when_making_position_required`: rows inserted at version 5 survive the rebuild unchanged. It passes once 3.2 is done; confirm it fails against a rebuild that skips the copy. Verify: the test passes.
- [x] 3.3 Change `PlaylistVideo.position` to `i64`, replace the two constructors with `create(.., position, now)`, update the SQLite repository mapping and ordering, and delete its no-position tests. Fix every compile error (reconciler, downloader, test seeds). Verify: `cargo test --locked` passes.

## 4. Verification

- [x] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass.
- [x] 4.2 Apply 0006 with `sqlite3` to a scratch copy of the prod DB and confirm the migration applies, the row count holds and per-playlist order is unchanged. Done this way instead of through `scripts/run-local.sh`, because `serve` on a prod copy would start reconciling: YouTube calls, deleting local files and Plex writes. Result: version 5 → 6, 1197 → 1197 rows, identical order hash, `integrity_check` ok.
- [x] 4.3 `openspec validate require-playlist-video-position --strict` passes.
