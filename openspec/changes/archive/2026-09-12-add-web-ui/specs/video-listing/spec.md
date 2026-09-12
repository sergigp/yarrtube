## Purpose

Provides an HTTP endpoint to list the videos belonging to a single tracked playlist, so a playlist's contents can be browsed.

## ADDED Requirements

### Requirement: List Videos For A Playlist
The system SHALL provide an HTTP endpoint that, given a tracked playlist's ID, returns every video recorded for that playlist, including each video's ID, title, status, quality (when downloaded), and filename (when downloaded).

#### Scenario: Playlist has videos
- **WHEN** a client requests the videos of a tracked playlist that has one or more recorded videos
- **THEN** the daemon responds with HTTP status 200 and a list containing each of those videos

#### Scenario: Playlist has no videos
- **WHEN** a client requests the videos of a tracked playlist that has no recorded videos
- **THEN** the daemon responds with HTTP status 200 and an empty list

#### Scenario: Playlist does not exist
- **WHEN** a client requests the videos of a playlist ID that is not currently tracked
- **THEN** the daemon responds with HTTP status 400 and does not return a list
