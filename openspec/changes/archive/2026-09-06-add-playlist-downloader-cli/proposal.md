## Why

We want to download all videos in a YouTube playlist to a local folder without doing it manually one by one. This is the first step toward a future system that tracks playlists/channels and downloads new videos automatically — but that automation isn't built yet. Right now we just need a minimal, working CLI prototype to validate the core mechanics: query the playlist via the YouTube Data API v3, then hand each video off to `yt-dlp` for the actual download.

## What Changes

- New Rust binary crate (`yarrtube`) with a single CLI command: `yarrtube <playlist_url> <output_path>`.
- Reads a YouTube Data API v3 key from a `.env` file (`YOUTUBE_API_KEY`); `.env` is added to `.gitignore` and never committed.
- Calls the YouTube Data API v3 `playlistItems.list` endpoint (paginating through `nextPageToken`) to resolve the playlist URL into an ordered list of `(video_url, title)` pairs.
- Invokes the user's locally installed `yt-dlp` binary as a subprocess, once per video, sequentially (no concurrency), downloading into `<output_path>`.
- If a single video's download fails (private, deleted, region-locked, etc.), logs the error to the console and continues with the next video rather than aborting the run; prints a final summary of successes/failures.
- Uses `yt-dlp`'s own default format selection for now (no quality/format flags). This is called out explicitly as a shortcut: a future change should expose format/quality as a config option.
- No persistence: no database, no record of what was already downloaded, no dedup across runs. Re-running the same playlist re-downloads everything. This is acceptable for the prototype and explicitly deferred.
- No automation/scheduling: the command is invoked manually, once, per run. Playlist/channel tracking and automatic recurring downloads are out of scope for this change.

## Capabilities

### New Capabilities
- `playlist-download`: resolving a YouTube playlist URL into its member videos via the YouTube Data API v3, and downloading each video via `yt-dlp` into a target directory, with per-video failure isolation.

### Modified Capabilities
(none — greenfield project)

## Impact

- New Rust crate/project (`Cargo.toml`, `src/`) — this repository currently has no code, so this establishes the initial project structure.
- New runtime dependency: locally installed `yt-dlp` binary, invoked as a subprocess (assumed present, not installed by this change).
- New external dependency: YouTube Data API v3, requiring a user-supplied API key with a public data quota (`playlistItems.list` calls consume quota; large playlists page through multiple calls).
- New local file: `.env` holding `YOUTUBE_API_KEY`, added to `.gitignore`.
- No changes to existing systems (none exist yet in this repo).
