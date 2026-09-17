## Why

Video cards and rows across Home, playlist, and channel views show no sense of how long a video runs, and nothing in the SPA gives a channel a visual identity beyond its name (not the sidebar, not a video card, not the channel's own detail view) — both make a growing library harder to scan and recognize at a glance.

## What Changes

- `yt-dlp`'s per-video download call gains an additional `--print` line for duration; `Video::mark_downloaded` records it as `duration_seconds`, following the same "recorded fact, present only once the video has downloaded" pattern already used for `filename`/`thumbnail_filename`.
- `GET /playlists/{id}/videos`, `GET /channels/{handle}/videos`, and `GET /videos/recent` responses gain `duration_seconds` per video (`null` until downloaded); the SPA renders it on every video card/row in Home, PlaylistDetail, and ChannelDetail.
- Channel creation additionally resolves the channel's avatar image URL from the same YouTube Data API call already used to resolve its title (`channels?part=id,snippet`), downloads the image, and stores it under a new container-local `avatars/` directory — not mounted, not inside any channel's video storage path, so it never gets picked up by Plex. This mirrors the app's existing, accepted behavior that its SQLite database is itself container-local and rebuilt on every restart.
- A new `avatar_filename` field is recorded on the channel (mirroring `thumbnail_filename` on `Video`) and returned by every `channel-crud` endpoint. A new `/avatars` static HTTP mount serves the stored image, mirroring the existing `/media` mount.
- `GET /videos/recent`'s `source` object gains `avatar_filename` for channel sources, so Home cards can render the channel's avatar without an extra per-video lookup — the same zero-extra-query mechanism already used to thread `path` through.
- The SPA renders the channel avatar in the sidebar's channel list, on Home's video cards for channel-sourced videos, and next to the video title in `ChannelDetail`'s video detail panel.
- A channel's stored avatar file is deleted when the channel itself is deleted, mirroring how its downloaded video files are cleaned up on channel deletion.
- A failure to fetch or download a channel's avatar does not fail channel creation — the channel is created with no recorded avatar, mirroring how a missing video thumbnail does not fail a video download.

## Capabilities

### New Capabilities

- `channel-avatars`: resolving a channel's avatar image URL from YouTube, downloading and storing it locally, serving it over HTTP, and removing it when its channel is deleted.

### Modified Capabilities

- `channel-crud`: channel creation additionally resolves and records an avatar filename (best-effort, does not fail creation on its own); every channel response includes it.
- `video-download`: a video's duration, in seconds, is recorded whenever its download succeeds, following the same recorded-fact convention as its filename and thumbnail filename.
- `video-listing`: playlist, channel, and recent-video listing responses include each video's `duration_seconds`; the recent-videos endpoint's `source` object additionally includes `avatar_filename` for channel sources.

## Impact

**Backend:**
- `src/infrastructure/shared/ytdlp.rs` — one more `--print` line on the existing download invocation, plus parsing it out.
- `src/domain/video/video.rs` — `Video` gains `duration_seconds`; `mark_downloaded`/`reset_for_redownload` updated.
- `src/domain/services/video_downloader.rs` — threads the parsed duration through to `mark_downloaded`.
- `src/infrastructure/repositories/sqlite_video_repository.rs` — `videos` table gains a `duration_seconds` column.
- `src/application/http/videos/dto.rs` — `VideoResponse`, `RecentVideoResponse`, and `RecentVideoSourceResponse` gain the new fields.
- `src/domain/channel/channel.rs` — `Channel` gains `avatar_filename`.
- `src/infrastructure/repositories/youtube_channel_repository.rs` — parses `snippet.thumbnails` out of the response it already fetches.
- A new small infrastructure piece to download an image URL's bytes and write them under the local avatars directory, injected into `ChannelService` alongside the existing YouTube lookup port.
- `src/domain/channel/service.rs` — `create_channel` resolves and stores the avatar (best-effort); `delete_channel` removes the stored avatar file.
- `src/infrastructure/repositories/sqlite_channel_repository.rs` — `channels` table gains an `avatar_filename` column.
- `src/application/http/channels/dto.rs` — `ChannelResponse` gains `avatar_filename`.
- `src/serve.rs` — new `/avatars` `ServeDir` mount, an avatars root path, and wiring for the new avatar-storage dependency.

**Frontend:**
- `web/src/api.js` — a media-URL helper for avatars, alongside the existing `videoMediaUrl`.
- `web/src/components/Sidebar.jsx`, `Home.jsx`, `ChannelDetail.jsx`, `PlaylistDetail.jsx` — render the avatar and/or duration.
- `web/src/components/Thumbnail.jsx` — reused/extended for a circular avatar with the same missing-image fallback behavior video thumbnails already have.
- `web/src/App.css` — avatar and duration-badge styling.

**No Docker/README changes** are needed for the avatar path — it lives container-local and un-mounted, consistent with the app's existing, documented behavior that its database does the same.
