## Context

See proposal.md — Why. Current state that shapes the approach:

- `ytdlp::download_video` (`src/infrastructure/shared/ytdlp.rs`) invokes
  `yt-dlp` with `--quiet --no-warnings`. `--no-warnings` suppresses the exact
  line that announces SABR ("...formats have been skipped as they are missing
  a URL. YouTube may have enabled the SABR-only streaming experiment...").
- On a clean non-zero exit the process stderr is captured and returned as
  `DownloadAttempt::Failed { stderr }`; on success the process stderr
  (`RunOutcome::Success(output)`) is available but currently discarded.
- The recorded failure reason is built in `video_downloader.rs::record_failed`
  from `combined_reason(diagnosed, stderr)`, where `stderr` is the download's
  own captured stderr — today just the `ERROR:` line(s), because warnings are
  suppressed.
- All video-download logging (`warn!`/`info!`/`error!`) is emitted from the
  domain service `video_downloader.rs`, not from the infrastructure process
  layer.

## Goals / Non-Goals

**Goals:**
- Make the SABR-only condition detectable from logs on both a failed download
  and a degraded (lower-quality fallback) success.
- Keep the recorded failure reason, video status, and retry/exclusion behavior
  byte-for-byte what they are today.

**Non-Goals:**
- No change to the diagnostic probe's flags or its reason extraction.
- No change to requested formats/clients; no PO-token support.
- No attempt to quantify "how degraded" the fallback was (just that SABR was
  reported).

## Decisions

**1. Detect in the infrastructure layer, log in the domain layer.**
A pure function `sabr_reason(stderr: &str) -> Option<String>` lives in the
`ytdlp` module (it is a yt-dlp-output concern) and is unit-tested there. The
detected reason is threaded up through `DownloadAttempt` so the actual `warn!`
is emitted from `video_downloader.rs`, where every other video-download log
line already lives. Alternative (log directly from `ytdlp.rs`) rejected: it
would scatter domain-level observability into the process-invocation layer and
bypass the test-silence conventions the domain service follows.

**2. Carry the detected reason on both attempt variants.**
`DownloadAttempt::Succeeded(DownloadedVideo { .., sabr_notice: Option<String> })`
and `DownloadAttempt::Failed { stderr, sabr_notice: Option<String> }`, both
computed once from the captured stderr inside `download_video`. This keeps the
domain service free of yt-dlp marker-string knowledge and gives the
degraded-success path a first-class signal. Alternative (re-scan `stderr` in
the domain service) rejected: duplicates the marker knowledge and only covers
the failure path.

**3. Remove `--no-warnings` from the download invocation only; keep `--quiet`.**
`--quiet` still yields the clean `--print` stdout (duration + filename); only
`--no-warnings` gated the SABR line. The probe is untouched (Decision/Non-Goal
above). 

**4. Filter `WARNING:` lines out of the recorded failure reason.**
Removing `--no-warnings` means warning lines now reach the download's stderr,
which would otherwise pollute the recorded failure reason (which is built from
that stderr). So the stderr returned in `DownloadAttempt::Failed` for the
*recorded reason* excludes `WARNING:`-prefixed lines (preserving today's
ERROR-only reason), while the SABR scan runs over the *full, unfiltered*
stderr. This is what keeps the spec's "recorded failure reason unchanged"
guarantee true.

**5. Marker match.**
`sabr_reason` matches case-insensitively on the stable token `sabr` and
returns the trimmed matching `WARNING:` line(s) as the reason text. `SABR` is
specific enough not to false-positive on unrelated output, and keying on the
whole phrase would be brittle to minor wording changes by yt-dlp.

## Risks / Trade-offs

- **Removing `--no-warnings` leaks other warning text to captured stderr** →
  Mitigated by Decision 4: the recorded failure reason filters warnings out;
  only our own SABR scan reads the full stderr. A focused test asserts the
  recorded reason for a SABR-triggered failure still equals the `ERROR:` text,
  not the warning.
- **Over- or under-matching the SABR marker** → Keyed on `sabr`
  (case-insensitive); unit tests cover the real yt-dlp phrasing, a clean run
  (no event), and an unrelated warning (no event).
- **Enum shape change ripples through tests** → `DownloadAttempt` is internal;
  updating its constructors/matches is mechanical and compiler-checked.

## Migration Plan

Pure code change: no data, schema, config, or API migration. Deploy is the
normal image rebuild; rollback is a revert (restores `--no-warnings` and drops
the event). No persisted state is affected.
