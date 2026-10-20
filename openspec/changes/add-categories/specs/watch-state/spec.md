## ADDED Requirements

### Requirement: Mark A Category Watched
The system SHALL provide an HTTP endpoint that, given a category's identifier, marks every downloaded video of every member of the category as watched, including every other stored copy of those videos. Videos that have not finished downloading SHALL be left unchanged. The endpoint SHALL respond with HTTP status 204. When no category has the identifier, the endpoint SHALL respond with HTTP status 404.

#### Scenario: Category marked watched
- **WHEN** a client marks the category "Chess" watched, and its channel and playlist members have downloaded, unwatched videos
- **THEN** the daemon responds with HTTP status 204, every downloaded video of both members is watched, and the category's unwatched count is 0

#### Scenario: Copies outside the category
- **WHEN** a video of a member of the category is also tracked by a source outside the category, and the category is marked watched
- **THEN** that other copy is watched too

#### Scenario: Videos still downloading
- **WHEN** a member of the category has a video that has not finished downloading, and the category is marked watched
- **THEN** that video stays unwatched

#### Scenario: Unknown category
- **WHEN** a client marks a category identifier that does not exist watched
- **THEN** the daemon responds with HTTP status 404
