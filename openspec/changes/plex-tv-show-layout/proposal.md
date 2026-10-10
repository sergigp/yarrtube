## Why

Plex collections cannot give a good playback experience: a collection is a
grouping feature, not a playable container (its Play button always starts at
the first item, it has no resume or On Deck, autoplay between members is
inconsistent across clients, and its poster is an auto-collage of member
posters cropped to 2:3), so yarrtube's one-collection-per-channel/playlist
model is clunky no matter how well it converges. Plex's native TV Shows model
gives all of that for free (16:9 episode thumbnails, Continue Watching, play
next episode, a real poster per show), and it is the layout yarrtube used
before collections. Issue #89 asks for exactly this.

## What Changes

- **BREAKING** On-disk layout becomes a Plex/Kodi TV-show layout, for every
  player and with no layout flag: each tracked channel or playlist directory
  is a show (`tvshow.nfo`, `poster.jpg` from the channel avatar) and each
  video is an episode in a `Season <year>` folder named
  `S<year>E<MMDDii> - <title>.mp4` with its `.nfo` and `.jpg` next to it. The
  per-video folder and `movie.nfo` go away.
- Episodes are numbered from the video's publish timestamp: season = publish
  year, episode = publish month and day plus a two-digit same-day index.
  Channels and playlists use the same rule; the number is assigned once and
  never changes, so files never rename after they exist and Plex watch state
  (keyed by show + season + episode) stays on the right video.
- **BREAKING** The Plex collections integration is removed: no sync task, no
  matching/re-matching, no collection deletion, and the
  `YARRTUBE_PLEX_PLAYLIST_SECTION_ID`, `YARRTUBE_PLEX_CHANNEL_SECTION_ID` and
  `YARRTUBE_PLEX_RECONCILE_INTERVAL_SECONDS` variables disappear.
- The Plex folder scan stays, as its own optional integration enabled by
  `YARRTUBE_PLEX_URL`, `YARRTUBE_PLEX_TOKEN` and `YARRTUBE_PLEX_VIDEOS_PATH`:
  after a download yarrtube asks Plex to partially scan the video's show
  folder, in every library whose locations contain it (sections are
  discovered, not configured).
- New `migrate-layout` CLI subcommand (with `--dry-run`) that moves an
  existing library into the new layout from the database alone: renames
  each downloaded video's file set, rewrites its NFO as an episode NFO,
  writes show NFOs and posters, and updates the recorded filenames. It is
  idempotent and resumable, and skips videos it cannot place (no metadata or
  missing file), which reconcile heals later.
- Web UI: video names already show titles; the only visible change is the
  detail pane, which shows the episode's folder instead of the raw file path.
- Docs rewritten: `PLEX.md` becomes the TV Shows library setup (NFO Series
  agent, Seasons setting, partial auto-scan, the migration procedure),
  `INSTALLATION.md` env vars, `ARCHITECTURE.md`, `.env.example`,
  `scripts/run-local.sh`, `CLAUDE.md`.

## Capabilities

### New Capabilities

- `tv-show-layout`: the show/season/episode directory layout, the episode
  numbering rule, the show NFO and poster, and what happens when a video's
  number cannot be assigned yet.
- `layout-migration`: the `migrate-layout` subcommand: dry run, what it
  moves, what it skips, idempotence and resumability.
- `plex-library-scan`: the optional Plex partial-scan integration kept from
  `plex-collections`, now section-less.

### Modified Capabilities

- `video-naming`: the derived name is the episode base name (number plus
  sanitized title) rather than a per-video folder; collision fallback is no
  longer needed because the episode number is unique within a show.
- `video-metadata`: the sidecar is an episode NFO named after the video file
  (not `movie.nfo` in its own folder) with episode fields (`season`,
  `episode`, `aired`); it is still written before the media file.
- `video-thumbnails`: the thumbnail-ahead fetch writes `<episode base>.jpg`
  into the season folder, so it needs the video's number too.
- `video-download`: output location is the season folder inside the show
  directory, concurrent downloads are distinguished by episode number, and a
  download finished for a deleted video removes its file set.
- `video-cleanup`: a removed video's file, thumbnail and NFO are deleted by
  base name; show files and season folders are never deleted with a video.
- `playlist-reconciliation` and `channel-video-sync`: filesystem
  reconciliation treats show files, season folders and in-flight episode
  base names as protected, and sweeps orphans by base name.
- `channel-avatars`: the avatar is also copied into the channel's show
  directory as its poster.
- `web-ui`: the video detail pane shows the episode's folder, not the file
  path.

### Removed Capabilities

- `plex-collections`: every requirement is removed (the scan requirement
  moves to `plex-library-scan`).

## Impact

- Code: `domain/video` (episode number value object, filename derivation,
  output entries), `domain/video_metadata` (episode and show NFO rendering),
  `domain/services` (video downloader, thumbnail fetcher, internal video
  reconciler, video file deleter, a show metadata writer, the migration
  service), `infrastructure/shared/ytdlp.rs` (output template, publish
  timestamp), repositories for metadata, video files and avatars, `main.rs`
  (subcommand), `serve.rs` (Plex wiring reduced to the scanner), removal of
  `reconcile_plex_collections_task`, `plex_collection_reconciler`,
  `plex_collection_deleter`, the two `delete_plex_collection_on_*`
  subscribers, `domain/plex` collection types and the collection half of
  `plex_collection_repository`.
- Database: a migration adding the episode number columns to `videos`.
- API: unchanged shapes; `filename` and `thumbnail_filename` now contain
  `Season <year>/<base>.<ext>`.
- Web: `VideoDetail` path line only.
- Users: must run `migrate-layout` once after upgrading, then create a TV
  Shows library in Plex (Plex NFO Series agent) and delete the old Movies
  library; Plex watch history from the old library does not carry over.
