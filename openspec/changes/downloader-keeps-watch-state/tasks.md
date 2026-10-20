# Tasks

## 1. Walking skeleton

- [x] 1.1 Add `Video::with_watch_state_of` (returns `self` unchanged) and the private `VideoDownloader::save` (plain `video_repository.update`, returning the video). Route `record_downloaded`, `exclude` and `mark_errored` through `save`.

  Done when: `cargo build` and `cargo test --locked` pass, with no new behaviour and no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_keep_the_watch_state_recorded_while_downloading`: a successful download keeps the watch state stored mid-download.
- [x] 2.2 `it_should_keep_the_watch_state_recorded_while_a_failed_download_ran`: an errored-retrying save keeps it.
- [x] 2.3 `it_should_keep_the_watch_state_recorded_while_downloading_an_excluded_video`: an excluded save keeps it.
- [x] 2.4 `it_should_keep_the_watch_state_recorded_while_downloading_if_placing_fails`: the placement-error path keeps it, plus the episode number. It may already pass, since that path re-reads the video; then keep it as a regression test.

## 3. Infrastructure adapters (TDD)

None: no adapter changes.

## 4. Verification

- [x] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass.
- [x] 4.2 `scripts/run-smoke-tests.sh` passes.

## Workflow follow-up

- Archive the change after review.
