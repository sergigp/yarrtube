## Purpose

Lets yarrtube tell a Plex server to partially scan a show folder as soon as a video lands there, so new episodes appear in Plex immediately instead of on Plex's next scheduled library scan.

## ADDED Requirements

### Requirement: Plex Scan Integration Is Optional And Off By Default
The system SHALL enable the Plex scan integration only when `YARRTUBE_PLEX_URL`, `YARRTUBE_PLEX_TOKEN` and `YARRTUBE_PLEX_VIDEOS_PATH` are all set to a non-empty value. `YARRTUBE_PLEX_VIDEOS_PATH` is the path at which the Plex server sees the directory yarrtube knows as `YARRTUBE_VIDEOS_PATH`. When any of the three is unset the system MUST NOT contact any Plex server. A partial configuration SHALL be logged as a warning at startup.

#### Scenario: Integration disabled
- **WHEN** the daemon starts with any of the three variables unset
- **THEN** no request is made to a Plex server

#### Scenario: Integration enabled
- **WHEN** the daemon starts with all three variables set
- **THEN** downloaded videos trigger Plex scan requests as described below

### Requirement: Show Folder Scanned After A Download
When the integration is enabled, the system SHALL, after each successful video download, ask Plex to scan the video's show directory (its source's output directory, mapped to its Plex-side path by replacing the `YARRTUBE_VIDEOS_PATH` prefix with `YARRTUBE_PLEX_VIDEOS_PATH`). The system SHALL discover the target libraries itself by listing the Plex server's library sections and SHALL request the scan on every section whose locations contain that Plex-side path. The request SHALL run asynchronously from the download: a failed or slow request MUST NOT affect the download's outcome, and a failed request SHALL be logged and retried a bounded number of times. A show directory outside `YARRTUBE_VIDEOS_PATH`, or contained by no section, SHALL be logged and skipped.

#### Scenario: Show folder scanned
- **WHEN** `YARRTUBE_VIDEOS_PATH` is `/videos`, `YARRTUBE_PLEX_VIDEOS_PATH` is `/volume1/media/yarrtube`, a Plex section has library location `/volume1/media/yarrtube/channels`, and a video of the channel stored at `channels/Some Channel` downloads
- **THEN** Plex is asked to scan `/volume1/media/yarrtube/channels/Some Channel` in that section

#### Scenario: Only containing sections are scanned
- **WHEN** a video's show directory lies under one section's location and not under another's
- **THEN** the scan is requested only on the first section

#### Scenario: No section contains the folder
- **WHEN** no Plex section's locations contain the show directory's Plex-side path
- **THEN** no scan is requested, and a warning is logged

#### Scenario: Scan request fails
- **WHEN** the scan request to Plex fails
- **THEN** the video is still recorded as downloaded, and the failure is logged and retried
