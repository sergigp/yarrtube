# Proposal

## Why

Future features (local LLM summaries and recommendations via Ollama) need to
know what each video talks about, and YouTube captions are the cheapest
source of that. The same captions, saved as a sidecar subtitle file, give
Plex/Kodi/Jellyfin subtitles for free.

## What Changes

- Every Downloaded video gets its original-language captions fetched by a
  new, separately scheduled subtitle fetch task (like thumbnails): uploaded
  captions preferred, YouTube's auto-generated captions in the original
  language as fallback. No translated tracks.
- The captions are written as an `.srt` next to the episode
  (`Season <s>/<episode base>.<lang>.srt`) and stored as a plain-text
  transcript (language + whether manual or auto-generated) in the database.
- Each video records a subtitle state: Missing, Fetched or Unavailable.
  Unavailable is terminal and only reached when the video has no
  original-language captions and is at least 7 days old (captions appear late
  on new uploads); any failure (rate limit, network) leaves it Missing.
- Every reconcile pass schedules a subtitle fetch for each Downloaded video
  whose subtitles are Missing, so the existing library is backfilled
  automatically after upgrading.
- Subtitle fetches run in their own task lane, one at a time, so a backfill
  of a large library never floods YouTube nor delays downloads, thumbnails
  or reconciles.
- The subtitle file is protected from the orphan sweep and deleted together
  with its video's other files when the video is removed.
- The tasks view describes subtitle fetches in plain language.

## Capabilities

### New Capabilities
- `video-subtitles`: fetching a video's original-language captions, writing
  them as an episode subtitle file, storing the plain-text transcript, and
  tracking the video's subtitle state (Missing / Fetched / Unavailable).

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

- Database: new subtitle state + subtitle filename on `videos`, new
  `video_transcripts` table (migration `0010`).
- `yt-dlp` integration: new caption probe and caption-only download
  invocations (requires the bundled `ffmpeg` for `--convert-subs srt`).
- Domain: `Video`, `EpisodePaths`, `InternalVideoReconciler`,
  `VideoFileDeleter`, the playlist/channel reconcilers (transcript deleted
  with its video), new subtitle fetcher service and transcript aggregate.
- Tasks: new `fetch_subtitles` task type and handler; `delete_video_file`
  payload gains the subtitle filename (older queued payloads without it still
  decode).
- Web UI: `web/src/lib/tasks.ts` describes and categorizes the new task.
- Upgrading: the first reconcile pass of every source queues one subtitle
  fetch per downloaded video; they drain one at a time.
