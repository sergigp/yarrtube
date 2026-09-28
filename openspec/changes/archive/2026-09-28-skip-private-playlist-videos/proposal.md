## Why

YouTube playlists can contain private and deleted videos. The playlist-items API returns them as items titled "Private video" / "Deleted video", reconcile stores them as PENDING, and their downloads fail. Through the errored-video recovery, they are retried every day, forever, and show up in the UI as "Downloading Private video".

## What Changes

- The YouTube playlist listing requests each item's privacy status (`part=snippet,status`) and only returns items whose `status.privacyStatus` is `public` or `unlisted`. Private, deleted, and any item with a missing or unrecognised status are dropped.
- Reconcile no longer sees those items, so it never stores them or schedules a download for them.
- Private/deleted videos already stored (PENDING, retrying or errored) are no longer in the listing, so the next reconcile pass removes them through the existing removed-video cleanup.
- A video that was downloaded and later becomes private on YouTube is treated the same as a video removed from the playlist: its record is deleted and its local file is removed. This is an accepted consequence.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `playlist-reconciliation`: only public and unlisted items of a YouTube playlist count as its members during the membership diff.

## Impact

- `src/infrastructure/repositories/youtube_playlist_items_repository.rs`: request the `status` part, deserialize `status.privacyStatus`, filter items.
- No domain, database, migration or HTTP API change. API quota cost is unchanged (`playlistItems.list` is 1 unit per page regardless of parts).
- Channels (synced via `yt-dlp --flat-playlist`) are out of scope.
