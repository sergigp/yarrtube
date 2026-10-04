## 1. Walking skeleton

- [x] 1.1 Add an empty `migrations/0006_playlist_video_position_required.sql` and register it in `sqlite_migrations::apply`. Verify: `cargo build` succeeds and `cargo test --locked` passes.

## 2. Behaviour (TDD)

- [x] 2.1 None new: no new observable behaviour.

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 `sqlite_migrations::it_should_keep_playlist_videos_when_making_position_required`: write the table rebuild in 0006. Verify: the test passes.
- [ ] 3.2 `sqlite_migrations::it_should_reject_a_playlist_video_without_position`. Verify: the test passes.
- [ ] 3.3 Change `PlaylistVideo.position` to `i64`, replace the two constructors with `create(.., position, now)`, update the SQLite repository mapping and ordering, and delete its no-position tests. Fix every compile error (reconciler, downloader, test seeds). Verify: `cargo test --locked` passes.

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass.
- [ ] 4.2 Run `scripts/run-local.sh` against a copy of the prod DB and confirm the migration applies and playlists list in order. Never run it against the prod file itself.
- [ ] 4.3 `openspec validate require-playlist-video-position --strict` passes.
