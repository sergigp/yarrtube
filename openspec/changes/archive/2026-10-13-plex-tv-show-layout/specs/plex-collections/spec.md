## REMOVED Requirements

### Requirement: Plex integration is optional and off by default
**Reason**: Collections are replaced by the TV-show layout (`tv-show-layout`); the only remaining Plex interaction is the folder scan, specified as `plex-library-scan` with its own enabling variables. `YARRTUBE_PLEX_PLAYLIST_SECTION_ID`, `YARRTUBE_PLEX_CHANNEL_SECTION_ID` and `YARRTUBE_PLEX_RECONCILE_INTERVAL_SECONDS` are removed.
**Migration**: Remove the three variables; keep `YARRTUBE_PLEX_URL`, `YARRTUBE_PLEX_TOKEN` and `YARRTUBE_PLEX_VIDEOS_PATH` to keep scan triggering.

### Requirement: One collection per tracked playlist and channel
**Reason**: Each tracked playlist and channel is now a Plex show; Plex provides the per-source tile, ordering, resume and autoplay natively.
**Migration**: Run `migrate-layout`, create a TV Shows library with the Plex NFO Series agent, delete the old Movies library (which deletes its collections).

### Requirement: Videos are matched to Plex items by YouTube ID
**Reason**: yarrtube no longer reads or edits Plex items.
**Migration**: None.

### Requirement: Collection membership converges to yarrtube's state
**Reason**: No collections are managed.
**Migration**: None.

### Requirement: Sync is decoupled from reconciliation
**Reason**: There is no Plex sync task any more.
**Migration**: None.

### Requirement: Per-collection failures do not abort the pass
**Reason**: There is no Plex sync pass any more.
**Migration**: None.

### Requirement: Deleting a playlist or channel deletes its collection
**Reason**: Deleting a playlist or channel deletes its show directory; Plex drops the show on its next scan.
**Migration**: None.

### Requirement: Downloaded videos are scanned into Plex
**Reason**: Moved to `plex-library-scan`, scanning the show folder and discovering sections instead of using configured section ids.
**Migration**: See `plex-library-scan`.

### Requirement: Unidentified Plex items are re-matched
**Reason**: Episode identity in Plex comes from the show NFO plus season and episode numbers on disk, so there are no unidentified items to repair.
**Migration**: None.
