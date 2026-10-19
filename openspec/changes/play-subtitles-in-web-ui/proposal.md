## Why

Every downloaded video already gets its original-language subtitles as a sidecar `.srt` for Plex, but the web UI player shows none: it has no caption track and the API doesn't say which videos have subtitles. Browsers only play WebVTT captions, so the stored SRT can't be pointed at directly.

## What Changes

- New HTTP endpoints serve a video's fetched subtitles as WebVTT, converted from the stored `.srt` when requested. No new files are written, so existing videos work immediately with no backfill, and the library layout Plex/Jellyfin/Kodi read stays unchanged:
  - `GET /api/playlists/{id}/videos/{video_id}/subtitles.vtt`
  - `GET /api/channels/{handle}/videos/{video_id}/subtitles.vtt`
- Playlist and channel video listings report each video's subtitle language (`subtitle_language`), present only once its subtitles are fetched.
- The web UI player offers the video's subtitles as a captions track labelled with the language name (e.g. "English"). The track is off by default and the user turns it on through the player's native captions control.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `video-subtitles`: adds serving a video's fetched subtitles as WebVTT over HTTP.
- `video-listing`: listed videos include their subtitle language once fetched.
- `web-ui`: the detail view's player offers the selected video's subtitles as an opt-in captions track.

## Impact

- Backend: new domain service and WebVTT conversion, a read method on the video file port, two routes in `src/application/http/mod.rs`, a `subtitle_language` field on `VideoResponse`.
- Web: `Video` type and fixture builder, a subtitles URL helper, `VideoPlayer` plus the playlist/channel detail views that render it, and a language-label helper in `src/lib/`.
- Smoke tests: the new routes must be exercised, because route coverage is enforced.
- No DB migration and no new dependency.
