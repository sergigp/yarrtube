# video-search Specification

## Purpose

Lets the web UI's global search find downloaded videos of tracked channels and playlists by title, so a user can jump straight to a video without browsing its source.

## Requirements

### Requirement: Search Videos By Title
The daemon SHALL provide `GET /api/videos/search?q=<text>` returning `{ "videos": [...] }`: the downloaded videos of tracked channels and tracked playlists, home exclusions ignored, whose title contains the trimmed text, ignoring ASCII case. Each YouTube video SHALL appear once, through a tracking channel when there is one, otherwise through a tracking playlist. Entries SHALL have the same fields as home videos (identifier, title, thumbnail, duration, watched state, playback position, source, category identifiers and publish date). The response SHALL list at most 20 videos, newest sync first.

#### Scenario: Matching titles
- **WHEN** a client searches "tactics" and the downloaded videos "Chess Tactics: Spot the Trigger" and "Endgame Skills" exist
- **THEN** the response lists "Chess Tactics: Spot the Trigger" and not "Endgame Skills"

#### Scenario: Case ignored
- **WHEN** a client searches "TACTICS"
- **THEN** the response lists the same videos as for "tactics"

#### Scenario: Accents not folded
- **WHEN** a client searches "falcon" and the only matching-looking title is "Víctor Falcón explica"
- **THEN** the response does not list it

#### Scenario: Video tracked twice
- **WHEN** a matching video is tracked by the channel "Dr. Can's Chess Clinic" and by a playlist
- **THEN** it is listed once, with the channel as its source

#### Scenario: Not downloaded
- **WHEN** a video whose title matches has not been downloaded
- **THEN** it is not listed

#### Scenario: Excluded playlist
- **WHEN** a matching video is tracked only by a playlist excluded from home
- **THEN** it is listed, with that playlist as its source

#### Scenario: Bounded results
- **WHEN** 30 downloaded videos match
- **THEN** the response lists the 20 most recently synced

### Requirement: Search Text Validation
The daemon SHALL reject a search whose trimmed text is shorter than 2 characters, or missing, with HTTP status 400.

#### Scenario: Too short
- **WHEN** a client searches " a "
- **THEN** the daemon responds with HTTP status 400

#### Scenario: Missing text
- **WHEN** a client calls the search endpoint without `q`
- **THEN** the daemon responds with HTTP status 400
