## Why

Production logs show three defects since the Plex collections sync shipped.
First, every reconcile pass gets `404 Not Found` from Plex for 26 of ~56
collections: adding a video to an existing collection calls an endpoint Plex
does not expose. As a result, new channel/playlist videos never reach their
collections.

Second, the hourly missing-thumbnail recovery keeps re-running `yt-dlp` for
videos that can never succeed. Examples are the copyright-taken-down Yakari
episodes and a members-only Big Think video, which are already `EXCLUDED`
for download. That is ~15 pointless `yt-dlp` runs per hour.

Third, those runs write yt-dlp's raw `ERROR:` lines straight into the
daemon's output, with no timestamp, level, or video id.

## What Changes

- Fix the Plex collection membership calls: add items via
  `PUT /library/collections/{key}/items` and remove one via
  `DELETE /library/collections/{key}/items/{ratingKey}` (currently
  `/library/metadata/...`, which 404s). There is no requirement change:
  the existing "Collection membership converges" requirement already
  mandates this behavior, and the code violated it.
- Missing-thumbnail recovery (playlists and channels) no longer schedules a
  fetch for a video whose download status is `Excluded` (permanently
  unavailable) or `Errored` (gave up; recovery resets it for redownload
  later, and that download writes its own thumbnail).
- A thumbnail fetch task that runs for an `Excluded` or `Errored` video
  makes no changes and does not invoke `yt-dlp`. This covers fetches
  already queued before the video reached that status.
- The thumbnail fetch and the channel video listing capture `yt-dlp`'s
  stderr instead of inheriting it. The relevant failure reason is reported
  through the structured logger, attached to the existing warn message, so
  no raw subprocess output reaches the daemon's log stream.
- A failed channel listing (`yt-dlp` non-zero exit) is now an error
  carrying the tool's reason, instead of an empty listing. Today the
  empty listing makes the reconciler evict every stored video of that
  channel, deleting the files. The reconciler treats that error as
  non-fatal: it logs it with the channel and reason, skips membership
  changes for that pass, and still runs the rest of the pass and schedules
  the next reconcile. It doesn't fail the task, which would dead-letter the
  channel's recurring reconcile for good. That matches the existing
  `channel-video-sync` scenario "yt-dlp fails to list a channel's videos",
  so there is no requirement change.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `playlist-reconciliation`: "Missing Thumbnail Recovery" stops scheduling
  fetches for `Excluded`/`Errored` videos.
- `channel-video-sync`: "Missing Thumbnail Recovery (Channels)" stops
  scheduling fetches for `Excluded`/`Errored` videos.
- `video-thumbnails`: "Thumbnail Fetched Ahead Of Video Download" adds
  that a fetch running for an `Excluded`/`Errored` video makes no changes.
- `logging`: new requirement that external-process diagnostics (yt-dlp
  stderr) are reported through leveled, structured log messages, never
  written raw to the daemon's output.

## Impact

- `src/infrastructure/repositories/plex_collection_repository.rs`:
  `add_items`/`remove_item` paths and their HTTP-mock tests.
- `src/domain/services/thumbnail_fetcher.rs`: the `schedule_missing` filter
  and the `fetch` guard.
- `src/application/tasks/fetch_thumbnail_task.rs`: covered by the `fetch`
  guard, with tests.
- `src/domain/services/channel_video_reconciler.rs`: a failed listing is
  logged and skipped for the pass instead of propagated.
- `src/infrastructure/shared/ytdlp.rs`: `fetch_thumbnail` and
  `list_channel_videos` switch from `Stdio::inherit()` to captured stderr.
  The callers log the reason.
- No API, DTO, schema, or config changes. After deploy, the next Plex pass
  adds the backlog of missing members to the 26 lagging collections.
