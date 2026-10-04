## Context

See proposal.md for why. Current state relevant to the approach:

- `VideoDownloader::download` runs `yt-dlp` via
  `VideoDownloaderRepository::download`, which resolves the per-video folder
  (`prepare_video_dir`: reuse the folder a thumbnail fetched ahead already
  created, or create a fresh collision-free one) and runs `yt-dlp` inside it.
  Only after `record_downloaded` does `generate_metadata` fetch YouTube
  metadata and `VideoMetadataRepository::save` write `movie.nfo` + the DB row.
- `yt-dlp` output is `<folder>.%(ext)s`, so the thumbnail is predictably
  `<folder>.jpg` (`expected_thumbnail_filename`).
- `PlexCollectionRepository::list_items` keeps only items with a
  `youtube://` GUID; unidentified items are silently dropped.
- Production evidence (Plex on the NAS): unidentified items have top-level
  guid `local://<ratingKey>`; `GET /library/metadata/<key>/matches` for one of
  them returns a single candidate
  `tv.plex.agents.nfo.movie://movie/youtube_<id>` built from its `movie.nfo`.
  `GET /library/sections` returns each section's `Location` paths
  (`/volume1/data/media/yarrtube/playlists`, `…/channels`), while the
  container mounts `/volume1/data/media/yarrtube` at `/videos`.
- Managed Plex sections hold only yarrtube videos, so every item in them is
  ours.

## Goals / Non-Goals

**Goals:**
- `movie.nfo` is in place before the mp4 appears, for every download whose
  metadata fetch works.
- New downloads reach Plex through an explicit partial scan of their folder.
- Already-unidentified items converge without manual Plex work.

**Non-Goals:**
- Staging downloads outside the library or hiding `yt-dlp` intermediates
  (`.part`, `.fNNN.mp4`) from Plex. With `movie.nfo` already present, an
  early scan of an intermediate still matches correctly.
- Inferring the Plex-side path automatically from matched items.
- Re-matching items in sections yarrtube doesn't manage.

## Decisions

### 1. Metadata first, then download; re-save after
`VideoDownloader::download` will:
1. fetch YouTube metadata + playlist position (the existing
   `generate_metadata` inputs), keeping it in memory;
2. resolve the video folder (new `VideoDownloaderRepository::prepare_folder`,
   wrapping `prepare_video_dir`); `download` keeps its signature and receives
   the resolved folder as its `existing_folder`, so `yt-dlp`'s own cleanup
   never removes it, and the downloader itself removes a folder it freshly
   prepared when the download doesn't succeed (today's "remove only a fresh
   folder" rule, moved up one layer);
3. if metadata was fetched, write `movie.nfo` only (new
   `VideoMetadataRepository::write_nfo`, no DB row) with `thumb` =
   `<folder>.jpg`;
4. run `yt-dlp`;
5. on success, `save` as today (rewrites `movie.nfo` with the real thumbnail
   or none, and records the row). If step 1 failed, fetch once more here,
   which is today's path. On failure, delete `movie.nfo` from the folder
   (best effort).

Why: the NFO must exist before the mp4, and only the folder name is needed
to write it. Recording the row only after success keeps "metadata recorded"
meaning "for a downloaded video", so the reconcile backfill keeps working.
Alternative: download into a hidden staging folder and move mp4 + nfo in
atomically. That is stronger but adds a cross-folder move and new cleanup
paths, and the remaining race (metadata fetch failure) is handled by
decision 3 anyway.

### 2. Folder scan via a `VideoDownloaded` domain event
`VideoDownloader` publishes `VideoDownloaded { video_id, output_dir, folder }`
after recording a successful download. A new subscriber
`scan_plex_folder_on_video_downloaded`, registered only when Plex is enabled
**and** `YARRTUBE_PLEX_VIDEOS_PATH` is set, maps the container folder to the
Plex-side path (strip the `YARRTUBE_VIDEOS_PATH` prefix, join onto
`YARRTUBE_PLEX_VIDEOS_PATH`; outside the root → debug log and skip), lists
section locations (`GET /library/sections`), and calls
`GET /library/sections/{id}/refresh?path=<plex path>` on each configured
section whose location is a path-prefix of it. When no configured section
contains it, it logs a warning and succeeds.

Why: domain events give async isolation plus the existing 5-attempt retry
and dead-letter for free, and the downloader stays Plex-agnostic, in line
with "Sync is decoupled from reconciliation". Alternatives: a new
`Task::ScanPlexFolder` (equivalent, but needs another task type and lane
choice for no gain), or calling Plex inline from the downloader (couples
downloads to Plex availability).

The event payload carries `output_dir` + `folder` rather than a full path so
it stays meaningful if other subscribers appear.

### 3. Self-heal unidentified items in the sync pass
`PlexItem.youtube_video_id` becomes `Option<String>`, and `list_items`
returns every item. The reconciler partitions each section's items: for each
item without a YouTube ID it calls new `list_match_candidates(rating_key)`
(`GET /library/metadata/{key}/matches`), picks the candidate whose guid
starts with `tv.plex.agents.nfo.movie://` and contains `youtube_`, and calls
new `match_item(rating_key, guid, name)`
(`PUT /library/metadata/{key}/match?guid=…&name=…`), logging `info` on
success and `warn` on no candidate or failure. Collection convergence uses
only the identified items of the same listing, so a re-matched item joins
its collection on the next pass. This avoids re-listing a 2.4 MB section
mid-pass.

Why match over unmatch + refresh: the match endpoint is the "Fix Match" flow,
and it was checked against the real server (a single NFO candidate is
returned). Unmatch + refresh relies on background matching with undocumented
timing.

### 4. Timeout 10 s → 30 s
Keep one client-wide `REQUEST_TIMEOUT`, raised to 30 s. Normal listings take
about 1 s. Failures happened only during Plex Butler maintenance. 30 s still
bounds a hung server on the Light lane (passes run every 15–30 min).

## Risks / Trade-offs

- [The pre-download `movie.nfo` names `<folder>.jpg` when the thumbnail later
  fails] → step 5 rewrites `movie.nfo` without `thumb`; a dangling `thumb` in
  between is harmless to Plex.
- [A reused (thumbnail-ahead) folder keeps a stale `movie.nfo` if the
  best-effort delete on failure fails] → no media in the folder means Plex
  ignores it, and the next attempt overwrites it.
- [An item stuck with no YouTube candidate (e.g. its `movie.nfo` was never
  written) is retried every pass] → only a warn log and one cheap GET per
  item; it resolves once the reconcile backfill writes `movie.nfo`.
- [A wrong `YARRTUBE_PLEX_VIDEOS_PATH` makes every scan miss every section] →
  a warn log names the computed path, and Plex's own auto-scan and the
  self-heal still cover it.
- [Plex refresh is fire-and-forget: `refresh?path=` returns before the scan
  ends] → collection membership is still converged by the periodic pass.

## Migration Plan

Deploy the new image and add
`YARRTUBE_PLEX_VIDEOS_PATH=/volume1/data/media/yarrtube` to the container
env. The first sync pass re-matches the 10 existing `local://` items, and the
pass after that adds them to their collections. The one unscanned Bob el
manetes folder needs a one-off scan (Plex UI "Scan Library Files" with a
forced refresh, or touching the folder). Rollback: redeploy the previous
image. Re-matched items stay correctly matched.
