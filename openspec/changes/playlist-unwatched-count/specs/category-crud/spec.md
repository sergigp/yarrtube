# Spec Delta

## MODIFIED Requirements

### Requirement: List Categories
The system SHALL provide an HTTP endpoint that returns every category, ordered by name ignoring case, together with the unwatched total across all categories. Each category SHALL include its identifier, its name, its unwatched count, the handles of its channel members and the IDs of its playlist members. The unwatched total SHALL be the number of distinct YouTube videos counted by any category, each category counting as stated in Category Unwatched Count.

#### Scenario: Categories listed by name
- **WHEN** the categories "science", "Chess" and "Sports" exist
- **THEN** the daemon responds with HTTP status 200 and lists them in the order "Chess", "science", "Sports"

#### Scenario: Members listed
- **WHEN** the category "Chess" has the channel "@GothamChess" and the playlist "PLendgames" as members
- **THEN** its listed channel members are ["@GothamChess"] and its listed playlist members are ["PLendgames"]

#### Scenario: Total counts a shared video once
- **WHEN** a playlist in "Chess" and a channel in "Science" both track the same downloaded, unwatched YouTube video, and nothing else is unwatched
- **THEN** the unwatched total is 1

#### Scenario: Total skips videos reached only through hidden playlists
- **WHEN** the only downloaded, unwatched video in any category belongs to a playlist member whose unwatched count is hidden
- **THEN** the unwatched total is 0

#### Scenario: No categories
- **WHEN** no category exists
- **THEN** the daemon responds with HTTP status 200, an empty list and an unwatched total of 0

### Requirement: Category Unwatched Count
A category's unwatched count SHALL be the number of distinct YouTube videos that are downloaded and unwatched among the videos of all its members. A YouTube video tracked by more than one member of the category SHALL count once. A video reached only through playlist members whose unwatched count is hidden SHALL NOT count; a video also reached through a channel member or a playlist member that shows its count SHALL count once.

#### Scenario: Count across a channel and a playlist
- **WHEN** a category's channel member has 2 downloaded, unwatched videos and its playlist member has 3 others
- **THEN** the category's unwatched count is 5

#### Scenario: Shared video counts once
- **WHEN** a category's channel member and playlist member both track the same downloaded, unwatched YouTube video, and nothing else is unwatched
- **THEN** the category's unwatched count is 1

#### Scenario: Videos not yet downloaded
- **WHEN** a category's members only have videos that are pending, downloading or failed
- **THEN** the category's unwatched count is 0

#### Scenario: Hidden playlist adds nothing
- **WHEN** a category's channel member has 1 downloaded, unwatched video and its playlist member, whose unwatched count is hidden, has 2 others
- **THEN** the category's unwatched count is 1

#### Scenario: Video of a hidden playlist also in a counted member
- **WHEN** a category's channel member and its playlist member whose unwatched count is hidden both track the same downloaded, unwatched YouTube video, and nothing else is unwatched
- **THEN** the category's unwatched count is 1
