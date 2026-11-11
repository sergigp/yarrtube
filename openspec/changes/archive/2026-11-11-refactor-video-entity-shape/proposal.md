# Proposal

## Why

`Video` (`src/domain/video/video.rs`, ~1100 lines, ~55 pub fns, 27 fields) is
flat. The download re-implements the per-step retry policy by hand instead of
using `StepState` (`is_download_due`, `next_download_at`, `mark_errored`), its
result lives in loose `Option`s that can contradict its status, each step's
value lives apart from its state under inconsistent names, and ~300 test
sites spell out flat fields, so every new field costs a sweep.

## What Changes

- `Video` stays one aggregate, one `videos` row, one `version`. Its fields are
  grouped into concrete value objects, one per concern, each with its own
  transitions: `Download` (+ `DownloadedFile`), `Thumbnail`, `Subtitles`,
  `Chapters`, `DownloadedQuality`, `WatchState`. `summary` and
  `categorization` stay `StepState` (their values live in other tables).
- No generic wrappers: each type is concrete and owns its rules, accepting
  some duplication. `LateDataChecks` is removed; `Subtitles`, `Chapters` and
  `DownloadedQuality` each hold their own `StepState` + `last_checked_at`.
- The download uses `StepState` (shared retry policy and backoff) plus a
  `started` flag for In Progress; its hand-written retry logic is deleted.
  `VideoStatus` becomes derived (`Download::status()`), API strings unchanged.
- `Video` keeps only transitions (writes, which stamp `updated_at`) and reads
  that span concerns; single-concern reads move to the concern
  (`video.download.is_due(now)`, `video.watch.is_watched()`).
- Test-only builders/fixtures move out of `video.rs` so the struct-update
  sweep shrinks.
- No schema migration, no stored-value change, no HTTP/DTO/web change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None: behaviour-preserving refactor (`skip_specs: true`).

## Impact

- `src/domain/video/`: `video.rs` shrinks; new `download.rs`,
  `downloaded_file.rs`, `thumbnail.rs`, `subtitles.rs`, `chapters.rs`,
  `downloaded_quality.rs`, `watch_state.rs`, `late_video_data.rs` (constants),
  `video_fixtures.rs` (`#[cfg(test)]`); `late_data_checks.rs` deleted.
- `src/infrastructure/repositories/sqlite_video_repository.rs`: row mapping
  only.
- Domain services and HTTP DTO mapping: field paths and moved reads.
- Tests: construction of seeded/expected videos only; no existing assertion
  changes. The audit added 5 tests for behaviour mutation testing showed
  unprotected (late-data retries, upgraded quality, subtitles discarded after
  a rename, rewatch at 90%).
- One PR, one commit per task group, each green. It also carries an
  incidental tooling commit (`scripts/run-tests.sh` and its docs).
