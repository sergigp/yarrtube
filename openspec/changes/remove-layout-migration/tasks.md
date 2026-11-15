## 1. Walking skeleton

- [x] 1.1 Remove the migration code and rewire, keeping the build and every remaining test green:
  - delete `src/application/cli/migrate_layout.rs`, the `MigrateLayout` variant and `pub mod migrate_layout` in `src/application/cli/mod.rs`, and its arm in `src/main.rs`;
  - delete `src/domain/services/layout_migrator.rs` and `src/domain/services/video_relocator.rs` with their `mod.rs` entries and re-exports;
  - drop `video_relocator` from `VideoReconciler` (field, `new` param) and `relocate_if_legacy`; `repair_metadata` saves metadata only for a video already in its season folder (no new legacy guard yet, task 2.1 adds it);
  - remove `VideoSyncChanges::relocated` and its log field, `Video::relocate_to_episode` (and its unit test), `MOVIE_NFO_FILENAME` (and its use in `sqlite_video_metadata_repository` tests), `serve::video_relocator` and its call sites;
  - delete `it_should_relocate_a_legacy_video_when_its_metadata_is_recovered` from `sync_video_task/tests.rs`.
  Done when `cargo build` and `cargo test --locked` pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_leave_a_legacy_video_without_metadata_untouched`: a sync of a Downloaded legacy video with no metadata, YouTube metadata available, writes no NFO or season folder, records no metadata and keeps its filename (`repair_metadata` returns early on `is_in_legacy_layout()`).

## 3. Infrastructure adapters (TDD)

None.

## 4. Docs

- [x] 4.1 Delete `doc/UPGRADING-TO-0.9.0.md`; remove its mention in `CLAUDE.md` (docs list and the `yarrtube migrate-layout` sentence), the `migrate-layout` line in `CLAUDE.md`'s "What this is", and the "Legacy layout" bullet's `migrate-layout` mention in `doc/ARCHITECTURE.md` (legacy files are still served, nothing moves them); point the CHANGELOG 0.9.0 entry's guide link and the 0.9.0 announcement's link in `announcements/announcements.json` at the guide as of the `v0.9.0` tag instead (there's no v0.9.0 GitHub release).
- [x] 4.2 Add a **Removed** line to the 1.0.0 CHANGELOG entry: the `migrate-layout` command, which moved 0.8.x libraries to the TV-show layout.

## 5. Verification

- [ ] 5.1 `scripts/run-tests.sh --lint --web --rust` passes.
- [ ] 5.2 `cargo run -- migrate-layout` fails with clap's unknown-subcommand error; `cargo run -- --help` no longer lists it.
