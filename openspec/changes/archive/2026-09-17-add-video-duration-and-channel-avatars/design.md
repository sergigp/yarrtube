## Context

See proposal.md - Why. Relevant existing shape:

- `ytdlp::download_video` (`src/infrastructure/shared/ytdlp.rs`) runs `yt-dlp` once per video with `--quiet --print after_move:filename` and reads the *last* stdout line as the saved filename. `VideoDownloader::download` (`src/domain/services/video_downloader.rs`) is the sole caller of `Video::mark_downloaded`, which already threads a `thumbnail_filename` recorded fact the same way `filename`/`quality` are recorded — see the `add-video-thumbnails` change for the precedent this mirrors.
- Channel creation (`ChannelService::create_channel`) calls `YoutubeChannelRepository::resolve` exactly once, which hits `GET .../youtube/v3/channels?part=id,snippet&forHandle=...` and currently only reads `snippet.title`. This is the one and only place a channel's YouTube-sourced fields are resolved — there is no periodic refresh on reconcile.
- Channel video *discovery* (`YtDlpChannelVideosRepository`) is separate from channel *creation* and goes through `yt-dlp --flat-playlist`, not the Data API — it never touches `channels.snippet`.
- The app's SQLite database is intentionally container-local and un-mounted (`/app/yarrtube.sqlite3`); the README documents that it — and therefore every channel/playlist row — is rebuilt from scratch on every container restart. Only `/videos` is a mounted, persistent volume.
- `/media` is an existing `ServeDir` mount rooted at the videos path (`src/serve.rs`), used today to serve both video files and their sibling thumbnail `.jpg` files via `videoMediaUrl(path, filename)` in the SPA.
- `VideoSearcher::list_recent` (backing `GET /videos/recent`) already loads the full `Playlist`/`Channel` per source to get its id, and the `add-video-thumbnails-to-listings` change threaded that source's `path` through `VideoSource`/`RecentVideoSourceResponse` at zero extra query cost — `avatar_filename` rides the same path.
- `videos`/`channels` tables are `CREATE TABLE IF NOT EXISTS` with no migration framework; per the `add-video-thumbnails` design note, a new column only needs to be safe on a freshly-created table, since the DB itself doesn't survive a restart.

## Goals / Non-Goals

**Goals:**
- Duration and avatar both follow the codebase's existing "resolved/recorded once, `None` when unavailable, never retroactively healed" convention — no new state-management pattern.
- No new required external dependency beyond what's already used (`yt-dlp`, `reqwest`, the YouTube Data API key already configured).
- No Docker/README changes.

**Non-Goals:**
- Backfilling duration for already-downloaded videos or avatars for already-created channels.
- Refreshing a channel's avatar or name after creation (matches existing behavior: title isn't refreshed on reconcile either).
- Healing an avatar file that goes missing from disk while its channel row stays intact (mirrors the accepted thumbnail-healing gap from `add-video-thumbnails`).
- Any change to what channel *video discovery* fetches (`YtDlpChannelVideosRepository` stays yt-dlp-based and untouched).

## Decisions

**Duration comes from a second `--print` line on the existing per-video `yt-dlp` invocation, prepended before the filename print.** `download_video` currently does `stdout.lines().next_back()` to get the filename, relying on `--print after_move:filename` being the only print directive. Adding `--print duration` (or `%(duration)s`) as an *earlier* argument keeps that filename parsing untouched (still the last line) and makes the new duration line the second-to-last. This was chosen over a separate `videos.list(part=contentDetails)` YouTube Data API call because: it reuses a call that already happens per video with no added network round trip or quota cost, it keeps one video-metadata pipeline instead of two, and channel-sourced videos don't go through the Data API at all today (only playlist discovery does), so a Data-API-based approach would need a second, different mechanism for channels. The accepted trade-off is that `duration_seconds` stays `None` until a video finishes downloading — identical to how `thumbnail_filename` already behaves, and the SPA already renders a blank-placeholder pattern for exactly this "not yet known" case.

**`duration_seconds` is a plain recorded fact on `Video`, following `filename`/`thumbnail_filename`.** Set by `mark_downloaded`, cleared by `reset_for_redownload`. No derived/computed alternative was considered seriously — duration isn't derivable from anything else already stored.

**Channel avatar URL is parsed from the existing `channels?part=id,snippet` response**, adding `snippet.thumbnails.<size>.url` to the `ChannelSnippet`/`ChannelItem` deserialization in `youtube_channel_repository.rs`. No new `part` value or second request needed. YouTube's `channels.list` typically offers `default`/`medium`/`high` thumbnail sizes; `medium` (240x240) is the right size for a UI avatar (sidebar row, a small card badge, a detail-view corner) without pulling down `high` unnecessarily — `default` is chosen as a fallback only if `medium` is absent, though in practice YouTube channel avatars provide all three.

**Avatar bytes are downloaded and written to a new container-local `avatars/` directory, not the channel's video storage path.** This directory lives next to `yarrtube.sqlite3` (i.e. relative to the app's working directory, not under the mounted `/videos` root), exactly matching the existing, accepted ephemeral-on-restart behavior of the database itself: when a channel row is rebuilt from YouTube after a restart, its avatar is naturally re-fetched too, so nothing is lost that wasn't already going to be lost. This was chosen over the alternatives raised during exploration — storing under the channel's video path (rejected: Plex would index it as media) and adding a new mounted volume (rejected: over-engineering for a value that's exactly as durable as the row it belongs to).

**Avatar filename convention: `<channel-handle>.<ext>`,** derived from the channel's own handle (already filesystem-safe — see `ChannelHandle`'s existing validation) rather than a generated ID, so the file is human-inspectable and naturally one-per-channel (a re-created channel with the same handle simply overwrites its old avatar file, which is fine since the old DB row is gone anyway). Extension is taken from the downloaded image's content type (YouTube avatars are effectively always `jpg`, but the code shouldn't hard-fail if a `png` shows up).

**A new small port, not a method on `YoutubeChannelRepository`, owns "fetch bytes from a URL and write them locally."** `YoutubeChannelRepository::resolve` stays a pure YouTube Data API client returning a `ResolvedChannel` (now including an optional avatar URL); a separate `ChannelAvatarRepository`-shaped port (fetch + local write, returning the stored filename or `None` on any failure) is injected into `ChannelService` alongside it. This keeps the existing repository's single responsibility (talk to the YouTube API) intact and matches the layering convention: an HTTP-fetch-then-local-write is an infrastructure concern orthogonal to "resolve channel metadata," the same way `VideoFileRepository` is orthogonal to the YouTube playlist-items lookup.

**Avatar fetch/download failures are swallowed to `None`, never surfaced as a `create_channel` error.** Mirrors `add-video-thumbnails`'s "thumbnail unavailable is not a download failure" scenario. A channel is a useful, functioning record without a picture; it shouldn't become impossible to track a channel because an image CDN hiccuped.

**Serving: a second `ServeDir` mount, `/avatars`, rooted at the same local avatars directory**, parallel to the existing `/media` mount. A dedicated mount (rather than folding avatars into `/media`) keeps the two storage roots — one mounted/persistent, one container-local/ephemeral — visibly distinct in both the router and the URL shape, and avoids `/media` needing to serve from two different roots depending on path shape.

**Channel avatar deletion is inline in `ChannelService::delete_channel`, not a subscriber on `ChannelDeleted`.** Unlike downloaded video files (cleaned up asynchronously via `ChannelDeleted` → a subscriber, because that cleanup can involve many files across a filesystem walk), deleting one small local avatar file is a single, fast, synchronous operation with no failure mode worth decoupling — same reasoning that already applies to deleting the channel's own DB rows inline rather than via an event.

## Risks / Trade-offs

- **Duration is `None` until a video actually finishes downloading** → Accepted; matches the existing thumbnail behavior and UI placeholder pattern, and avoids a second metadata-fetch mechanism split across playlist/channel sources.
- **A channel avatar is never refreshed after creation** (a channel that changes its picture keeps showing the old one, or none) → Accepted for this change, matching existing title behavior; noted as a Non-Goal rather than left implicit.
- **Avatar file is lost on container restart along with its channel row** → Accepted; this is a re-statement of the app's existing, documented database-ephemerality trade-off, not a new one introduced here.
- **Schema changes with no migration framework** → New `duration_seconds` column on `videos` and `avatar_filename` column on `channels`, both added to their respective `CREATE TABLE IF NOT EXISTS` statements, are safe for a freshly-created table — consistent with how `thumbnail_filename` was added previously.
- **`yt-dlp`'s duration print format** (plain seconds vs. `H:MM:SS`) needs confirming during implementation so the parsed value is always a plain integer second count, not a display string — if `%(duration)s` yields a float-looking value for some videos, it should be parsed and truncated to whole seconds rather than stored as-is.
