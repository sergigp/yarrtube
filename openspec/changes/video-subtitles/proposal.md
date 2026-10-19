# Proposal

## Why

Future features (local LLM summaries and recommendations via Ollama) need to
know what each video talks about, and YouTube captions are the cheapest
source of that. The same captions, saved as a sidecar subtitle file, give
Plex/Kodi/Jellyfin subtitles for free.

## What Changes

- Every Downloaded video gets its original-language captions fetched by a
  new, separately scheduled subtitle fetch task (like thumbnails), queued as
  soon as the video finishes downloading: uploaded captions preferred,
  YouTube's auto-generated captions in the original language as fallback.
  No translated tracks.
- The captions are written as an `.srt` next to the episode
  (`Season <s>/<episode base>.<lang>.srt`, `<lang>` the base language code,
  e.g. `.es.srt` for `es-US`, so media servers recognise it) and stored as a
  plain-text transcript (full language code + whether manual or
  auto-generated) in the database.
- When the Plex scan trigger is configured, the show folder is scanned
  again after a subtitle file is written, so Plex picks it up right away.
- Each video records a subtitle state: Missing, Fetched or Unavailable.
  Unavailable is terminal and only reached when warning-free checks found no
  original-language captions for at least 7 days, counted both from the
  publish date (captions appear late on new uploads) and from the first such
  check (so an old playlist added today still gets a week of retries); any
  failure (rate limit, network) or yt-dlp warning (it can silently withhold
  caption tracks) leaves it Missing. A redownload resets it to Missing.
- The `.srt` is named after the downloaded video file, not the current
  title, so it still pairs with the media file after a video is retitled.
- Every reconcile pass also schedules a subtitle fetch for each Downloaded
  video whose subtitles are still Missing, so the existing library is
  backfilled automatically after upgrading and videos whose captions
  weren't available yet are retried.
- Subtitle fetches run in their own task lane, one at a time, so a backfill
  of a large library never floods YouTube nor delays downloads, thumbnails
  or reconciles.
- The subtitle file is protected from the orphan sweep and deleted together
  with its video's other files when the video is removed.
- The tasks view describes subtitle fetches in plain language.

## Capabilities

### New Capabilities
- `video-subtitles`: fetching a video's original-language captions after
  its download, writing them as an episode subtitle file, storing the
  plain-text transcript, tracking the video's subtitle state (Missing /
  Fetched / Unavailable), and asking Plex to rescan after a subtitle file is
  written (configured as in `plex-library-scan`).

### Modified Capabilities
- `tv-show-layout`: a season folder may also hold the episode's `.<lang>.srt`
  subtitle file, sharing the episode base name.
- `playlist-reconciliation`: reconcile schedules missing subtitle fetches;
  the orphan sweep protects recorded subtitle files.
- `channel-video-sync`: same as `playlist-reconciliation`, for channels.
- `video-cleanup`: a removed video's subtitle file is deleted with its other
  files.
- `task-scheduling`: new subtitle lane with a concurrency of 1.
- `web-ui`: tasks view describes subtitle fetch tasks.

## Impact

- Database: new subtitle state, subtitle filename and first-check-without-captions
  time on `videos`, new
  `video_transcripts` table (migration `0010`).
- `yt-dlp` integration: new caption probe and caption-only download
  invocations (requires the bundled `ffmpeg` for `--convert-subs srt`).
- Domain: `Video`, `EpisodePaths`, `InternalVideoReconciler`,
  `VideoFileDeleter`, the playlist/channel reconcilers and the channel/playlist
  deleters (transcript deleted with its video), new subtitle fetcher service
  and transcript aggregate.
- Tasks: new `fetch_subtitles` task type and handler; `delete_video_file`
  payload gains the subtitle filename (older queued payloads without it still
  decode).
- Events: new `VideoSubtitlesFetched`; new subscribers
  `FetchSubtitlesOnVideoDownloaded` and `ScanPlexFolderOnVideoSubtitlesFetched`;
  `VideoRemovedFromPlaylist`/`VideoRemovedFromChannel` carry the subtitle
  filename (older events without it still decode).
- Web UI: `web/src/lib/tasks.ts` describes and categorizes the new task.
- Upgrading: the first reconcile pass of every source queues one subtitle
  fetch per downloaded video; they drain one at a time.
