# web-ui Specification

## Purpose

Serves a browsable single-page application from the daemon's own HTTP server, so tracked playlists, their videos, and in-flight tasks can be inspected from a browser without a separate deployment or a database client.

## Requirements

### Requirement: Serve The Application Shell
The system SHALL serve the single-page application's HTML page at `GET /`, without requiring authentication.

#### Scenario: Root request
- **WHEN** a client sends `GET /` to the running daemon
- **THEN** the daemon responds with HTTP status 200 and the application's HTML

### Requirement: Serve Application Assets
The system SHALL serve every static asset (script, stylesheet, and other build output) the application's HTML page references, without requiring authentication.

#### Scenario: Asset request
- **WHEN** a client requests one of the application's referenced asset files
- **THEN** the daemon responds with HTTP status 200 and that asset's content

### Requirement: Self-Contained Deployment
The system SHALL serve the application and its assets using only the running binary, without requiring any additional file, directory, or volume to be present at runtime.

#### Scenario: Fresh deployment with no extra files
- **WHEN** the daemon is started with only its binary present (no application source or build output mounted or copied alongside it)
- **THEN** requests to the application's root and its assets still succeed as in the scenarios above
