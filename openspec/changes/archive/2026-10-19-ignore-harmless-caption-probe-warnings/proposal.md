## Why

The caption probe treats any yt-dlp warning as inconclusive, so a video without captions stays Missing forever and is re-probed on every reconcile. In production almost every probe warns about things unrelated to captions (yt-dlp failing to write its cache, the SABR "formats have been skipped" notice): 108 inconclusive probes on the first day, 751 subtitle fetches for ~197 fetched.

## What Changes

- The caption probe ignores warnings known not to affect caption tracks: yt-dlp cache write failures and "formats have been skipped" notices.
- Any other warning still makes a probe without a selectable track inconclusive (video stays Missing, warning logged).

## Capabilities

### New Capabilities

### Modified Capabilities
- `video-subtitles`: Subtitle Fetch Outcome — a check is conclusive unless the downloader reports a warning that can affect caption tracks; cache write failures and skipped-formats notices no longer count.

## Impact

- `src/infrastructure/shared/ytdlp.rs` (probe warning collection + tests).
- `src/domain/services/subtitle_fetcher.rs` doc comment only.
- No API, DB, or config change.
