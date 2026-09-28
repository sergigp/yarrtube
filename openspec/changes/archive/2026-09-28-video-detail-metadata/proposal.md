## Why

The video detail pane shows little beyond the title: a status badge that almost always reads "Downloaded", a quality badge that is the same for every video in a source, a file path and a YouTube link. We already store each video's YouTube metadata (publish date, description, channel name) for Plex, and its sync time, but none of it reaches the user.

## What Changes

- Video listings for a playlist and for a channel include each video's publish time, description teaser (the stored 500-character plot) and channel name, each absent when the video has no generated metadata yet.
- The detail pane shows a meta line that is always visible, even when collapsed: `[<channel name> · ]Published <date> · Synced <relative time>`. The channel name appears only in playlist views, and missing parts are omitted.
- The expandable section shows the description teaser, clamped to about 4 lines with a "Show more" control that appears only when the text overflows. Line breaks are kept and URLs are clickable links.
- **BREAKING (UI)**: the quality badge is removed from the detail pane, and the status badge is shown only for videos that are not downloaded.
- Out of scope: full (untruncated) descriptions, clickable chapter timestamps, tags, genre, and duration in the pane.

## Capabilities

### New Capabilities

### Modified Capabilities
- `video-listing`: listed playlist and channel videos include publish time, description and channel name from their generated metadata.
- `web-ui`: the video detail pane gains an always-visible meta line and a description teaser; it drops the quality badge and shows the status badge only for videos that are not downloaded.

## Impact

- API: `GET` playlist and channel video listings gain three nullable fields (`published_at`, `description`, `channel_name`). The change is additive, and the recent-videos listing is unchanged.
- Backend: the video searcher also reads `video_metadata` when listing. There's no schema migration.
- Frontend: `VideoDetail.jsx` only. It already knows it's in a channel view because it receives a `channel` prop.
- Smoke tests: `waitForVideoStatus` currently waits for the "Downloaded" badge, which this change removes, so it needs a new signal that a video is downloaded.
