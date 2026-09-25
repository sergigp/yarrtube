## Why

The SPA plays downloaded videos but keeps no record of what has been watched. There is no way to tell which new channel uploads you haven't seen, a video you stopped halfway always restarts from zero, and nothing marks a video as watched. Tracking watch state per video makes the SPA usable as a daily viewer rather than only a file browser.

## What Changes

- A video gains a watch state: watched or not, plus a saved playback position. "Seen" and "watched" are the same concept.
- Watch state belongs to the YouTube video, not to one stored copy. Every stored copy of the same YouTube video (e.g. in a channel and in a "favorites" playlist) is updated together.
- New HTTP endpoints:
  - record playback progress for a video. The server marks the video watched once progress reaches 90% of its duration. A watched video is marked unwatched again once a rewatch passes 10%.
  - mark a channel watched, which marks every downloaded video in the channel as watched.
- Video listings (playlist, channel, recent) include each video's watch state.
- The channel listing is trimmed to the fields the SPA uses (handle, name, path, avatar filename) and adds each channel's count of downloaded, unwatched videos. **BREAKING** for any client reading `youtube_channel_id`, `quality`, `video_limit` or `created_at` from the list (the SPA does not).
- SPA:
  - resumes playback from the saved position and reports progress while playing (every 15s, and on pause, video switch and tab close).
  - shows a tick on watched video thumbnails (channel, playlist and home views).
  - shows an unwatched-count badge on channel rows in the sidebar.
  - adds a "mark channel watched" action to the sidebar row and to the channel view.
- A new SQLite migration adds the watch-state columns to `videos`. Existing videos start unwatched with no saved position.

## Capabilities

### New Capabilities

- `watch-state`: recording playback progress, the watched/unwatched rules (90% to mark watched, 10% into a rewatch to unmark), marking a whole channel watched, and sharing the state across every stored copy of a YouTube video. There is no single-video mark or unmark endpoint: progress covers both (seek past 90% to mark, rewatch past 10% to unmark).

### Modified Capabilities

- `video-listing`: listed videos (playlist, channel, recent) include their watch state.
- `channel-crud`: listed channels carry only handle, name, path and avatar filename, plus their count of downloaded, unwatched videos.
- `web-ui`: resume playback, progress reporting, watched ticks on thumbnails, the unwatched badge on sidebar channel rows, and the mark-channel-watched actions (sidebar row, channel view).

## Impact

- **Backend (Rust)**: `Video` entity (new fields and transitions), a new `PlaybackPosition` value object, `VideoRepository` (new columns, new lookup by YouTube ID), a new `VideoWatchStateUpdater` domain service, `ChannelSearcher` (flat `ChannelView` with the unwatched count), new HTTP handlers and routes, DTO fields on video responses and a new flat channel list response.
- **Database**: migration `0002` adds two columns and a `youtube_id` index to `videos`. It is additive, so existing data is kept.
- **API**: new fields on video responses; `GET /api/channels` trimmed to the fields above plus `unwatched_count`; new routes `POST /api/videos/{id}/progress` and `POST /api/channels/{handle}/watched`.
- **SPA**: `api.js`, the channel, playlist and home views, the sidebar, and a new playback-progress hook.
- **Smoke tests**: channel and playlist specs cover the new routes the UI calls.
