# Proposal

## Why

A download reads the video before it starts and saves the whole row from that old copy when it finishes, minutes later. A watch state recorded in between is lost without notice. This happens when another copy of the same YouTube video is marked watched, by marking a playlist, channel or category watched, or by playing it. The video comes back unwatched in its badges.

## What Changes

- When a download finishes, whether it succeeded, failed or ended excluded, the saved video keeps the watch state stored at that moment (`watched_at`, playback position, last played time), not the one read when the download started.
- The downloader re-reads the video just before each of those saves. There are no narrow repository writes and no transaction. Accepted residual: a mark-watched landing between that re-read and the save (sub-millisecond) can still be lost.
- The save when a download starts is unchanged, since it follows its read directly.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `watch-state`: "Watch State Per YouTube Video" gains a scenario: watch state survives a download that is in flight.

## Impact

- **Backend**: `VideoDownloader` (domain service) and `Video` (a new `with_watch_state_of` transition). The download task's tests.
- No API, schema, web or smoke changes.
