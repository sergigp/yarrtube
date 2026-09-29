## MODIFIED Requirements

### Requirement: List All Playlists
The system SHALL provide an HTTP endpoint that returns every currently stored playlist, sorted alphabetically by name, ignoring case.

#### Scenario: Playlists exist
- **WHEN** one or more playlists have been created
- **THEN** the system returns all of them, each with its ID, name, path, quality, kind, and creation timestamp

#### Scenario: No playlists exist
- **WHEN** no playlists have been created
- **THEN** the system returns an empty list rather than an error

#### Scenario: Playlists are sorted by name regardless of creation order
- **WHEN** playlists named "watch later", "Courses" and "Ambient" were created in that order
- **THEN** the system returns them in the order "Ambient", "Courses", "watch later"
