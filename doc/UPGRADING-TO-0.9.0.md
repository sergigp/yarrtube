# Upgrading to 0.9.0

This guide is for libraries downloaded by yarrtube 0.8.x or older. A fresh
install of 0.9.0 needs none of it.

## What changed

- **Layout**: older versions stored each video in its own folder with a
  `movie.nfo`, for a Plex Movies library. 0.9.0 lays every playlist and
  channel out as a TV show (see [PLEX.md](PLEX.md#what-you-get)).
- **Plex collections are gone**: yarrtube no longer manages Plex
  collections. Each playlist and channel is now its own show.
- **Plex settings**: `YARRTUBE_PLEX_PLAYLIST_SECTION_ID`,
  `YARRTUBE_PLEX_CHANNEL_SECTION_ID` and
  `YARRTUBE_PLEX_RECONCILE_INTERVAL_SECONDS` were removed. The only Plex
  feature left is the scan trigger, which is on only when
  `YARRTUBE_PLEX_URL`, `YARRTUBE_PLEX_TOKEN` **and**
  `YARRTUBE_PLEX_VIDEOS_PATH` are all set.

Upgrading does not touch your existing files: new downloads land in the TV
layout, old ones stay playable where they are until you migrate them once.
The migration **moves and renames** every downloaded video, which is why
you back up first.

## Steps

1. Back up your videos folder and database.
2. Stop the daemon and pull the new image:

   ```bash
   docker compose stop yarrtube
   docker compose pull yarrtube
   ```

3. Update the environment: remove the three variables above, and if you
   want the scan trigger make sure `YARRTUBE_PLEX_VIDEOS_PATH` is set (see
   [PLEX.md](PLEX.md#configure-yarrtube)).
4. Preview the migration. It prints every move (`from -> to`), every
   skipped video with its reason, and a summary. It moves no files; it
   only upgrades the database schema, as the daemon does on its next start:

   ```bash
   docker compose run --rm yarrtube yarrtube migrate-layout --dry-run
   ```

5. Run it for real:

   ```bash
   docker compose run --rm yarrtube yarrtube migrate-layout
   ```

   It writes every show's `tvshow.nfo` and `poster.jpg`, moves each
   downloaded video's file, thumbnail and NFO into its season folder,
   renames them to their episode name (e.g.
   `Season 2026/S2026E031501 - A video.mp4`), rewrites the NFO as an
   episode NFO and removes the old per-video folders. It exits non-zero if any video failed to move; the
   others are migrated anyway. Running it again is safe: it only moves what
   is still in the old layout.

6. Start the daemon again:

   ```bash
   docker compose up -d yarrtube
   ```

7. In Plex, create a TV Shows library on the same folders as described in
   [PLEX.md](PLEX.md#1-set-up-a-tv-shows-library), and delete the old
   Movies library (its collections go with it).

Videos the migration skips are healed by the daemon later: one with no
recorded metadata (so no publish time) is moved into its season folder as
soon as a reconcile pass manages to fetch its metadata, and one whose file
is missing is downloaded again.

**Watch history does not carry over** from the old Movies library: Plex
keys it by library item, and the episodes in the new library are new
items.
