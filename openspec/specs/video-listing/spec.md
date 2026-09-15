# video-listing Specification

## Purpose

Provides an HTTP endpoint to list the videos belonging to a single tracked playlist, so a playlist's contents can be browsed.

## Requirements

### Requirement: List Videos For A Playlist
The system SHALL provide an HTTP endpoint that, given a tracked playlist's ID, returns every video recorded for that playlist, including each video's ID, title, status, quality (when downloaded), and filename (when downloaded). For a YouTube-linked playlist, the returned videos SHALL be ordered by their current position in the source YouTube playlist. For a custom playlist, no particular order is guaranteed.

#### Scenario: Playlist has videos
- **WHEN** a client requests the videos of a tracked playlist that has one or more recorded videos
- **THEN** the daemon responds with HTTP status 200 and a list containing each of those videos

#### Scenario: Playlist has no videos
- **WHEN** a client requests the videos of a tracked playlist that has no recorded videos
- **THEN** the daemon responds with HTTP status 200 and an empty list

#### Scenario: Playlist does not exist
- **WHEN** a client requests the videos of a playlist ID that is not currently tracked
- **THEN** the daemon responds with HTTP status 400 and does not return a list

#### Scenario: YouTube-linked playlist's videos are ordered by playlist position
- **WHEN** a client requests the videos of a YouTube-linked playlist
- **THEN** the daemon returns them ordered by their position in the source YouTube playlist, matching the order they appear in on YouTube

### Requirement: List Videos For A Channel
The system SHALL provide an HTTP endpoint that, given a tracked channel's handle, returns every video recorded for that channel, including each video's ID, title, status, quality (when downloaded), and filename (when downloaded), ordered by recency (most recently uploaded first).

#### Scenario: Channel has videos
- **WHEN** a client requests the videos of a tracked channel that has one or more recorded videos
- **THEN** the daemon responds with HTTP status 200 and a list containing each of those videos, ordered most recent first

#### Scenario: Channel has no videos
- **WHEN** a client requests the videos of a tracked channel that has no recorded videos
- **THEN** the daemon responds with HTTP status 200 and an empty list

#### Scenario: Channel does not exist
- **WHEN** a client requests the videos of a channel handle that is not currently tracked
- **THEN** the daemon responds with HTTP status 400 and does not return a list
