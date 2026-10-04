## Why

The Plex collections integration matches items by the `youtube://` GUID that
Plex's NFO agent derives from `movie.nfo`, but Plex binds that GUID only the
first time it matches an item. Today the mp4 lands in its library folder
before `movie.nfo` is written (and when the post-download YouTube metadata
fetch fails, `movie.nfo` only appears hours later via the reconcile backfill),
so Plex's auto-scan can import the video first as an unidentified
`local://<n>` item that never gains a YouTube GUID. In production, 10 such
items are permanently invisible to the collection reconciler, so 7
collections are missing members. Separately, a video folder that Plex first
saw before its media existed (created by the thumbnail fetched ahead of the
download) was never revisited: Plex's scanner skips an unchanged parent folder
and never discovers the subfolder's later mp4. Finally, 5 of 35 sync passes
failed on a 10 s timeout while Plex was busy with its nightly maintenance.

## What Changes

- Downloads write `movie.nfo` into the video's folder **before** the media file
  appears there: YouTube metadata is fetched first, then the video is
  downloaded. Metadata generation stays best-effort and never fails the
  download; when the fetch fails, behavior falls back to today's (download
  anyway, backfill later), covered by the self-heal below.
- After a video downloads, when the Plex integration is enabled and the new
  `YARRTUBE_PLEX_VIDEOS_PATH` is set, yarrtube asks Plex to scan that video's
  folder (Plex partial scan), so new videos are imported promptly and
  deterministically rather than relying on Plex's filesystem watcher.
- New optional env var `YARRTUBE_PLEX_VIDEOS_PATH`: the path at which Plex
  sees yarrtube's videos root (`YARRTUBE_VIDEOS_PATH`). Without it, folder
  scans are skipped and everything else works as before.
- Each Plex sync pass self-heals unidentified items: an item in a managed
  section with no `youtube://` GUID is re-matched through Plex's match API to
  the NFO agent's candidate, so Plex re-reads `movie.nfo` and binds the
  YouTube GUID. This also repairs the 10 items already broken.
- Plex HTTP request timeout raised from 10 s to 30 s so section listings
  survive Plex's nightly maintenance window.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `video-metadata`: metadata is now fetched and `movie.nfo` written before the
  video's media file is downloaded into its folder (previously "immediately
  after"), and recorded as generated only once the download succeeds.
- `plex-collections`: new optional `YARRTUBE_PLEX_VIDEOS_PATH`; a downloaded
  video's folder is scanned into Plex; sync passes re-match managed-section
  items lacking a YouTube GUID.

## Impact

- Code: `domain/services/video_downloader.rs` (metadata-first order, publish a
  `VideoDownloaded` domain event), `infrastructure/shared/ytdlp.rs` /
  `youtube_video_downloader_repository.rs` (resolve the video folder before
  the download), `domain/event/domain_event.rs` (new event),
  `domain/services/plex_collection_reconciler.rs` (self-heal step),
  `infrastructure/repositories/plex_collection_repository.rs` (section
  locations, partial scan, match candidates, match, timeout), a new Plex
  folder-scan subscriber, `serve.rs` (env var, wiring).
- Plex API calls added: `GET /library/sections`,
  `GET /library/sections/{id}/refresh?path=…`,
  `GET /library/metadata/{key}/matches`,
  `PUT /library/metadata/{key}/match?guid=…&name=…`.
- Config/docs: `YARRTUBE_PLEX_VIDEOS_PATH` in `README.md`, `doc/PLEX.md` and
  the docker-compose example. No change for users who leave it unset.
