## Why

When YouTube enables its SABR-only streaming experiment for the daemon's
session, `yt-dlp` can no longer obtain a plain download URL for the better
formats: it skips them and either fails the video outright ("This video is not
available") or silently falls back to a low-quality/throttled format (e.g.
format 18, 360p). `yt-dlp` announces this on its standard error as a warning
("...formats have been skipped as they are missing a URL. YouTube may have
enabled the SABR-only streaming experiment..."), but the daemon runs `yt-dlp`
with `--no-warnings` on both the download and the diagnostic probe, so the
signal is discarded. In production this condition is therefore invisible: it
surfaces only as opaque per-video failures and unexplained slowness, and an
operator has no fast way to recognize SABR as the cause.

## What Changes

- Stop suppressing `yt-dlp`'s SABR-only warning on the download invocation so
  the signal reaches the daemon.
- Detect SABR-only markers in `yt-dlp`'s standard-error output and emit a
  dedicated, greppable warn-level event carrying the video id and the detected
  client/reason text, so the condition is immediately recognizable in logs.
- Surface the event for **both** outright download failures **and** degraded
  successes — a video that downloaded but fell back to a lower-quality format
  because higher formats were SABR-skipped must still be flagged.
- Scope is observability only: no change to download client selection, format
  selection, retry/exclusion behavior, or the introduction of PO-token support.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `video-download`: add a requirement that the system detects `yt-dlp`'s
  SABR-only streaming signal during a download attempt and surfaces it as a
  distinct warn-level event (with the video id and detected reason as fields),
  on both a failed download and a successful-but-degraded one. No change to
  which formats/clients are requested or to status/retry behavior.

## Impact

- Code: `src/infrastructure/shared/ytdlp.rs` (download invocation flags and
  stderr capture/classification on success and failure),
  `src/domain/services/video_downloader.rs` (emitting the SABR event). A small
  pure SABR-marker classifier, unit-tested in isolation.
- Behavior: slightly more `yt-dlp` warning text may reach captured stderr on
  the download path; the recorded failure reason and all video statuses are
  unchanged.
- No API, schema, dependency, or configuration changes.
