## 1. SABR marker classifier (infrastructure)

- [x] 1.1 Add a pure `sabr_reason(stderr: &str) -> Option<String>` to the
  `ytdlp` module that returns the trimmed `WARNING:` line(s) when the output
  mentions `sabr` (case-insensitive) and `None` otherwise. Verify with unit
  tests covering: the real yt-dlp SABR phrasing (Some, with the line text), a
  clean run (None), and an unrelated `WARNING:` line (None).
- [x] 1.2 Add a helper that strips `WARNING:`-prefixed lines from stderr for
  the recorded failure reason, leaving `ERROR:`/other lines intact. Verify
  with a unit test that a mixed warning+error stderr yields only the error
  text.

## 2. Thread the signal through the download invocation

- [x] 2.1 Add `sabr_notice: Option<String>` to `DownloadedVideo` and to
  `DownloadAttempt::Failed`. Update constructors/matches so the crate compiles
  (`cargo build`).
- [x] 2.2 Remove `--no-warnings` from the `download_video` invocation (keep
  `--quiet`); leave the diagnostic probe and thumbnail/listing invocations
  untouched. Verify via the existing captured-args test(s) updated to assert
  `--no-warnings` is absent from the download args and still present nowhere it
  was removed from by mistake.
- [x] 2.3 In `download_video`, compute `sabr_reason` from the full captured
  stderr on both the success and clean-failure paths, populate `sabr_notice`,
  and build the `Failed { stderr }` recorded reason from the
  warning-filtered stderr (Task 1.2). Verify with unit tests: a success whose
  stderr carries the SABR warning yields `Succeeded { sabr_notice: Some(..) }`;
  a clean failure with SABR warning + an `ERROR:` line yields
  `Failed { stderr: Some(<error text only>), sabr_notice: Some(..) }`.

## 3. Emit the event (domain service)

- [x] 3.1 In `video_downloader.rs`, when a download attempt carries a
  `sabr_notice`, emit a distinct warn-level event (stable message, with
  `video_id` and `reason` as structured fields) on both the succeeded and
  failed paths — independent of the existing success/failure logging and
  without altering status, recorded reason, or retry/exclusion flow. Verify by
  review against the spec scenarios (status/reason unchanged) plus the existing
  downloader tests still passing unchanged.

## 4. Verify end to end

- [x] 4.1 Run `cargo test --locked`, `cargo fmt --all -- --check`, and
  `cargo clippy --all-targets --all-features --locked -- -D warnings`; all
  pass.
- [x] 4.2 Confirm against the spec: a SABR-degraded success and a SABR failure
  each produce exactly one greppable SABR warn event with the video id, while a
  non-SABR download produces none and no recorded failure reason changes.
