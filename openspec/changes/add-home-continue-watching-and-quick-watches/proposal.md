## Why

The home view only shows "Latest videos", so a video you started yesterday or a short one you could watch right now is buried among everything else. Two sections that use data we already record (watch position, duration) make the home view more useful without new YouTube data.

## What Changes

- Record when each video was last played: every progress report stores a "last played" time on every stored copy of the video. No play time was recorded before, so existing part-watched videos are backfilled as played at upgrade time: they show up immediately and drop out after a week unless played again.
- New endpoint listing **continue watching** videos: downloaded, unwatched, playback position over 30 seconds, last played within the past 7 days, most recently played first, one entry per YouTube video. Videos not played for a week drop out, since they're probably not worth finishing.
- New endpoint listing **quick watches**: downloaded, unwatched videos shorter than 15 minutes, newest sync first, one entry per YouTube video.
- Both endpoints take the same optional `limit` as the recent videos endpoint (default 20, capped at 100) and return the same card shape, with the saved playback position added.
- The home view shows three sections, in this order: "Continue watching" (cards show a progress bar), "Latest videos" (unchanged), "Quick watches". The two new sections are hidden when empty.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `watch-state`: recording playback progress also records the last played time for every stored copy.
- `video-listing`: adds the continue watching and quick watches listings across sources.
- `web-ui`: the home page layout gains the "Continue watching" and "Quick watches" sections.

## Impact

- **Database**: migration `0005` adds `videos.last_played_at` (nullable) and sets it to the migration time for part-watched, unwatched videos.
- **Domain**: `Video` gains `last_played_at` and the continue-watching / quick-watch rules. `VideoSearcher` gains two listings and a clock dependency (for the 7-day window).
- **Infrastructure**: `VideoRepository` gains `find_many`, so listing across sources loads each source's videos in one query instead of one per video (this also speeds up the recent videos listing).
- **HTTP API**: new `GET /videos/continue-watching` and `GET /videos/quick-watches`. The recent videos card response gains `position_seconds` (additive).
- **Web**: `Home.jsx` renders three sections, plus a progress bar on continue-watching cards. `api.js` gains two fetchers.
- **Smoke tests**: the playlist spec part-plays its video and checks it appears under "Continue watching" on the home view, with a progress bar.
