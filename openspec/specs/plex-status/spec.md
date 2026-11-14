# plex-status Specification

## Purpose
Reports how the Plex integration is doing, from what its syncs last saw, and lets a user run a Plex sync right away, so its health can be checked without reading logs.

## Requirements

### Requirement: Plex Syncs Record Their Runs
Each finished watch sync and label sync SHALL record when it finished and either its error or, when it reached Plex, the Plex Media Server version and the titles of the libraries holding yarrtube's folders. Only the latest run of each sync SHALL be kept, and it SHALL survive a restart.

#### Scenario: Sync reaches Plex
- **WHEN** a watch sync finishes against Plex `1.43.5.11079` with yarrtube's folders in "Subscriptions" and "Sergi's Playlists"
- **THEN** its latest run records the finish time, no error, version `1.43.5.11079` and those two libraries

#### Scenario: Plex unreachable
- **WHEN** a label sync can't reach Plex
- **THEN** its latest run records the finish time and the error, and no version or libraries

#### Scenario: Restart
- **WHEN** a watch sync has run and the daemon restarts
- **THEN** its latest run is still reported

### Requirement: Plex Status Endpoint
The daemon SHALL provide `GET /api/plex`, responding with HTTP 200, whether Plex is configured, the connection as the most recent run of either sync saw it, and per sync (`watch_state`, `labels`) its latest run, its next run time and whether it is running. It MUST NOT contact Plex.

#### Scenario: Not configured
- **WHEN** any of `YARRTUBE_PLEX_URL`, `YARRTUBE_PLEX_TOKEN` and `YARRTUBE_PLEX_VIDEOS_PATH` is unset and a client requests `GET /api/plex`
- **THEN** the response says Plex isn't configured, with no connection and no syncs

#### Scenario: Configured, never synced
- **WHEN** Plex is configured and no sync has finished yet
- **THEN** the response has no connection, and each sync has no latest run and its next run time

#### Scenario: Latest run failed
- **WHEN** the watch sync reached Plex at 10:00 and the label sync failed to reach it at 10:05
- **THEN** the connection reads unreachable with the label sync's error at 10:05, and the version and libraries are those seen at 10:00

#### Scenario: Sync running
- **WHEN** a label sync is running and a client requests `GET /api/plex`
- **THEN** the label sync reads as running

#### Scenario: Plex down when the page asks
- **WHEN** the Plex server is unreachable and a client requests `GET /api/plex`
- **THEN** the daemon answers at once from the recorded runs

### Requirement: Sync A Plex Sync Now
The daemon SHALL provide `POST /api/plex/syncs/{kind}`, `kind` being `watch_state` or `labels`, responding with HTTP 204 and making that sync due now: its pending run moves to now, or one is scheduled now when none is pending. It MUST NOT leave more than one pending run of that sync.

#### Scenario: Pending sync brought forward
- **WHEN** the next watch sync is due in 4 minutes and a client sends `POST /api/plex/syncs/watch_state`
- **THEN** the daemon responds 204, the watch sync runs now, and only one is pending afterwards

#### Scenario: Requested twice
- **WHEN** a client sends `POST /api/plex/syncs/labels` twice in a row
- **THEN** one label sync runs, not two

#### Scenario: Requested while running
- **WHEN** a label sync is running and a client sends `POST /api/plex/syncs/labels`
- **THEN** another label sync runs right after it

#### Scenario: Unknown sync
- **WHEN** a client sends `POST /api/plex/syncs/scan`
- **THEN** the daemon responds with HTTP 404

#### Scenario: Plex not configured
- **WHEN** Plex isn't configured and a client sends `POST /api/plex/syncs/watch_state`
- **THEN** the daemon responds with HTTP 409 and schedules nothing
