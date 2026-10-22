## Why

Scheduled premieres (and live streams) show up in a channel's `/videos`
listing and in playlists before they can be downloaded. Yarrtube stores them
as PENDING and queues a download, which fails with "Premieres in N hours".
The retry backoff can run out before the premiere starts (seen in prod:
`sceZ1prgUNU`, last retry 13:08 UTC, premiere 14:00 UTC). The video then
stays errored, because reconciles never re-queue a video that is already
stored.

## What Changes

- Channel discovery ignores entries that yt-dlp reports as not yet
  watchable (`live_status` of `is_upcoming`, `is_live` or `post_live`): they
  are not stored and no download is queued for them.
- YouTube-linked playlist discovery ignores items that YouTube reports as an
  upcoming or currently live broadcast (`liveBroadcastContent` of `upcoming`
  or `live`, looked up with `videos.list` for each page of playlist items).
- An ignored video counts as absent from the listing. A video already
  stored while upcoming (like the errored one in prod) is therefore evicted
  or removed by the existing rules. Once it becomes watchable, the first
  reconcile adds it as a normal new video.
- Positions are taken from the filtered listing, so ignoring an entry
  doesn't leave a gap. For channels, an ignored entry still uses one of the
  `video_limit` slots that yt-dlp is asked for.

## Capabilities

### New Capabilities

### Modified Capabilities
- `channel-video-sync`: discovery skips upcoming, live and post-live entries.
- `playlist-reconciliation`: membership excludes upcoming and live broadcasts.

## Impact

- `src/infrastructure/shared/ytdlp.rs`: the flat-playlist parse reads
  `live_status` and filters on it.
- `src/infrastructure/repositories/youtube_playlist_items_repository.rs`: one
  extra `videos.list` call per page of 50 items (1 quota unit each, which
  roughly doubles playlist listing quota).
- No schema, API or UI changes.
