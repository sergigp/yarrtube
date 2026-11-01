# Proposal

## Why

Many YouTube videos split into named chapters (games of a match, parts of a
course, podcast segments), and `yt-dlp` exposes them in a fixed shape
(`[{start_time, end_time, title}]`). Yarrtube drops them today, so neither
the web player nor Plex/Kodi/Jellyfin can offer chapter navigation. Storing
them now lets a follow-up change add chapter navigation to the web player.

## What Changes

- Record each downloaded video's chapters (start, end, title), read from
  the same `yt-dlp` run that downloads it. "Not checked yet" is kept
  distinct from "checked, has no chapters".
- Embed the chapters into the downloaded mp4 (`--embed-chapters`), so
  Plex, Kodi and Jellyfin show them for videos downloaded from now on.
- Backfill: a video sync schedules a chapters fetch (a `yt-dlp` metadata
  read, no download) for a Downloaded video whose chapters were never
  checked, so the existing library gets its chapters over the next
  reconcile passes.
- The playlist, channel and category video listings return each video's
  chapters (`[]` when it has none or they were not checked yet). Web API
  types and test fixture builders are updated to match. No player UI.
- The Tasks page describes the new chapters fetch task ("Fetching
  chapters of <video> in <source>", under Downloads), since the backfill
  queues one per downloaded video.
- Out of scope: the player UI (follow-up change), adding chapters to files
  that were downloaded before this change.

## Capabilities

### New Capabilities
- `video-chapters`: recording a video's chapters from `yt-dlp`, at
  download time and through a separate chapters fetch for videos never
  checked, and embedding them into the downloaded file.

### Modified Capabilities
- `video-sync`: a video sync schedules a chapters fetch for a Downloaded
  video whose chapters were never checked.
- `video-listing`: the playlist, channel and category video listings
  include each video's chapters.
- `task-listing`: a chapters fetch task resolves its video title and
  playlist/channel name.
- `web-ui`: the tasks view describes a chapters fetch task and lists it
  with the downloads.

## Impact

- DB: new migration adding nullable `videos.chapters` (JSON text).
- Rust: `Video` entity and `SqliteVideoRepository`; `ytdlp::download_video`
  (new `--print`, `--embed-chapters`) and a new chapters-only `yt-dlp`
  call on `VideoDownloaderRepository`; `VideoDownloader`; new
  `ChapterFetcher` service and `fetch_chapters` task; `VideoSyncPlan`;
  video DTOs.
- Web: `web/src/api/types.ts` and `web/src/test/helpers.tsx` builders.
- Backfill cost: one `yt-dlp` metadata call per already-downloaded video,
  run through the task queue.
