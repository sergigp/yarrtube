## ADDED Requirements

### Requirement: Downloaded videos are scanned into Plex
When the Plex integration is enabled and `YARRTUBE_PLEX_VIDEOS_PATH` is set to
a non-empty value, the system SHALL, after each successful video download,
ask Plex to scan that video's folder. `YARRTUBE_PLEX_VIDEOS_PATH` is the path
at which the Plex server sees the directory yarrtube knows as
`YARRTUBE_VIDEOS_PATH`; the folder's Plex-side path SHALL be that value joined
with the folder's path relative to `YARRTUBE_VIDEOS_PATH`. The scan SHALL be
requested on every configured playlist or channel section whose Plex library
locations contain that Plex-side path. The scan request SHALL run
asynchronously from the download: a failed or slow request MUST NOT affect
the download's outcome, and a failed request SHALL be logged and retried a
bounded number of times. When `YARRTUBE_PLEX_VIDEOS_PATH` is unset, the Plex
integration is disabled, or the folder lies outside `YARRTUBE_VIDEOS_PATH`,
no scan is requested.

#### Scenario: Folder scanned after a download
- **WHEN** the Plex integration is enabled, `YARRTUBE_VIDEOS_PATH` is `/videos`, `YARRTUBE_PLEX_VIDEOS_PATH` is `/volume1/media/yarrtube`, a configured playlist section has library location `/volume1/media/yarrtube/playlists`, and a video downloads into `/videos/playlists/kids/Some video`
- **THEN** Plex is asked to scan `/volume1/media/yarrtube/playlists/kids/Some video` in that playlist section

#### Scenario: Section not containing the folder is left alone
- **WHEN** a video downloads into a folder whose Plex-side path lies under a channel section's location and not under any playlist section's location
- **THEN** the scan is requested only on that channel section

#### Scenario: Plex videos path not configured
- **WHEN** the Plex integration is enabled but `YARRTUBE_PLEX_VIDEOS_PATH` is unset and a video downloads
- **THEN** no scan request is sent to Plex and the download completes as before

#### Scenario: Scan request fails
- **WHEN** a video downloads and the scan request to Plex fails
- **THEN** the video is still marked as downloaded, and the failure is logged

### Requirement: Unidentified Plex items are re-matched
A sync pass SHALL, for every item in a configured section that carries no
`youtube://` GUID, ask Plex for its match candidates and, when Plex's NFO
agent offers a candidate derived from a YouTube ID, match the item to that
candidate so Plex re-reads the item's `movie.nfo` and binds its YouTube GUID.
Each re-match SHALL be logged. An item with no such candidate, or whose
re-match fails, SHALL be logged and skipped without failing the pass. Items
re-matched in a pass SHALL be eligible for collection membership no later
than the next pass.

#### Scenario: Item imported before its movie.nfo is re-matched
- **WHEN** a sync pass runs and a configured section holds an item with no `youtube://` GUID whose folder now has a `movie.nfo` with a YouTube `uniqueid`
- **THEN** the pass matches the item to the NFO agent's YouTube-derived candidate, and a later pass places it in its playlist's/channel's collection

#### Scenario: No YouTube candidate available
- **WHEN** a sync pass runs and Plex offers no YouTube-derived match candidate for an item lacking a `youtube://` GUID
- **THEN** the item is logged and left unchanged, and the pass continues

#### Scenario: Identified items untouched
- **WHEN** a sync pass runs and every item in a section carries a `youtube://` GUID
- **THEN** no match-candidate or match requests are made for that section
