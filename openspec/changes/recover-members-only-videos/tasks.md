# Tasks

## 1. Walking skeleton

- [x] 1.1 Add `Unavailability` (with `is_recheckable`), `DownloadFailure::unavailability`, `Download::unavailable(&DownloadFailure)`, `Download::is_recheckable_unavailable` and `Video::mark_unavailable(&DownloadFailure, now)` with the signatures from design.md. Replace `is_permanent()` callers with `unavailability().is_some()` and update every `mark_unavailable` call site and fixture. Stub bodies: `unavailability` keeps today's token matching, mapped to the variants; `is_recheckable` returns `false`; `is_recheckable_unavailable` returns `false`; `unavailable` still clears `failure`. Done when `cargo build` succeeds and all existing tests pass.

## 2. Behaviour (TDD)

- [ ] 2.1 `it_should_keep_the_reason_when_marking_the_video_unavailable`: `Download::unavailable` keeps the failure reason. Update the `download_failure.rs`/`download.rs` unit tests and fixtures that assumed it was cleared.
- [ ] 2.2 `it_should_schedule_a_download_for_a_members_only_unavailable_video`: `is_recheckable` for MembersOnly/Private, `is_recheckable_unavailable`, and `is_due` covering it.
- [ ] 2.3 `it_should_schedule_a_download_for_an_unavailable_video_without_a_reason`: a legacy Unavailable with no reason counts as recheckable.
- [ ] 2.4 `it_should_not_schedule_a_download_for_a_permanently_unavailable_video`: a permanent reason stays settled.
- [ ] 2.5 `it_should_download_a_members_only_unavailable_video_that_became_public`: `VideoDownloader::is_settled` lets a recheckable Unavailable download through.
- [ ] 2.6 `it_should_keep_a_still_members_only_video_unavailable_without_counting_an_attempt`: a repeated members-only failure leaves the video Unavailable with 0 attempts and the task settled.
- [ ] 2.7 `it_should_settle_a_private_video_that_is_now_removed`: the new reason replaces the old one, and a later task attempts nothing.
- [ ] 2.8 `it_should_not_attempt_a_permanently_unavailable_video`: no download call for a permanent Unavailable.
- [ ] 2.9 `it_should_retry_a_members_only_video_that_fails_transiently`: a transient failure on a recheckable Unavailable becomes Retrying with 1 attempt.

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 SqliteVideoRepository: `it_should_round_trip_an_unavailable_download_with_its_reason`.

## 4. Verification

- [ ] 4.1 `scripts/run-tests.sh --lint --web --rust` passes.
- [ ] 4.2 Update `doc/` wherever it says Unavailable/members-only videos are never retried (grep `doc/` and `README.md`), and verify the wording matches the specs.
- [ ] 4.3 Manual (human): on a real install, a members-first video from a channel goes from Unavailable to Downloaded within one reconcile pass after it goes public.
