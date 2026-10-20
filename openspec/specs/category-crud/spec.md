# category-crud Specification

## Purpose

Lets the user group tracked channels and playlists into their own categories (N-N), so content can be organised and watched by topic across sources.

## Requirements

### Requirement: Category Name
A category name SHALL be the given text with leading and trailing whitespace removed. It SHALL NOT be empty after trimming and SHALL be at most 60 characters long. Two category names SHALL be the same name when they are equal ignoring case.

#### Scenario: Name is trimmed
- **WHEN** a category is created with the name "  Chess  "
- **THEN** its name is "Chess"

#### Scenario: Blank name
- **WHEN** a category is created or renamed with an empty or blank name
- **THEN** the daemon responds with HTTP status 400 and records nothing

#### Scenario: Name too long
- **WHEN** a category is created or renamed with a name longer than 60 characters after trimming
- **THEN** the daemon responds with HTTP status 400 and records nothing

### Requirement: Create Category
The system SHALL provide an HTTP endpoint that creates a category from a name. A new category SHALL have no members and a system-generated identifier. The endpoint SHALL respond with HTTP status 201 and the created category: its identifier, its name and its (empty) member lists. When another category already has the same name, the endpoint SHALL respond with HTTP status 409 and create nothing.

#### Scenario: Category created
- **WHEN** a client creates a category named "Chess" and no category has that name
- **THEN** the daemon responds with HTTP status 201 and a category named "Chess" with an identifier and no members

#### Scenario: Name already taken
- **WHEN** a client creates a category named "chess" and a category named "Chess" exists
- **THEN** the daemon responds with HTTP status 409 and no category is created

#### Scenario: Missing name
- **WHEN** a client creates a category without a name
- **THEN** the daemon responds with HTTP status 400 and no category is created

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

### Requirement: Rename Category
The system SHALL provide an HTTP endpoint that, given a category's identifier and a name, renames the category and responds with HTTP status 204. Renaming SHALL keep the category's identifier and members. When another category already has the same name, the endpoint SHALL respond with HTTP status 409 and change nothing. Renaming a category to a name differing from its own only in case SHALL be allowed. When no category has the identifier, the endpoint SHALL respond with HTTP status 404.

#### Scenario: Category renamed
- **WHEN** a client renames the category "Chess" to "Board games"
- **THEN** the daemon responds with HTTP status 204 and the category, with the same identifier and members, is named "Board games"

#### Scenario: Changing only the case
- **WHEN** a client renames the category "chess" to "Chess"
- **THEN** the daemon responds with HTTP status 204 and the category is named "Chess"

#### Scenario: Name taken by another category
- **WHEN** a client renames "Chess" to "sports" and a category named "Sports" exists
- **THEN** the daemon responds with HTTP status 409 and both categories are unchanged

#### Scenario: Unknown category
- **WHEN** a client renames a category identifier that does not exist
- **THEN** the daemon responds with HTTP status 404

### Requirement: Delete Category
The system SHALL provide an HTTP endpoint that, given a category's identifier, deletes the category and its memberships and responds with HTTP status 204. Deleting a category SHALL NOT delete, change or stop tracking any channel, playlist or video. When no category has the identifier, the endpoint SHALL respond with HTTP status 404.

#### Scenario: Category deleted
- **WHEN** a client deletes the category "Chess", which has two channel members
- **THEN** the daemon responds with HTTP status 204, the category is no longer listed, and both channels are still tracked with their videos

#### Scenario: Unknown category
- **WHEN** a client deletes a category identifier that does not exist
- **THEN** the daemon responds with HTTP status 404

### Requirement: Set Category Members
The system SHALL provide an HTTP endpoint that, given a category's identifier, a list of channel handles and a list of playlist IDs, replaces the category's members with exactly those channels and playlists, and responds with HTTP status 204. A source repeated in a list SHALL be a member once. Every given channel and playlist SHALL be tracked; otherwise the endpoint SHALL respond with HTTP status 400 naming the first untracked source and change nothing. Empty lists SHALL leave the category with no members. A source SHALL be able to be a member of any number of categories. When no category has the identifier, the endpoint SHALL respond with HTTP status 404.

#### Scenario: Members added
- **WHEN** a client sets the members of the empty category "Chess" to the channels "@GothamChess" and "@Hikaru" and the playlist "PLendgames"
- **THEN** the daemon responds with HTTP status 204 and "Chess" has exactly those three members

#### Scenario: Members replaced
- **WHEN** "Chess" has the channels "@GothamChess" and "@Hikaru" and a client sets its members to the channel "@Hikaru" only
- **THEN** "Chess" has the single member "@Hikaru"

#### Scenario: A source in several categories
- **WHEN** a client adds the channel "@Veritasium" to "Science" while it is a member of "Chess"
- **THEN** "@Veritasium" is a member of both categories

#### Scenario: Untracked source
- **WHEN** a client sets the members of "Chess" to include a channel handle no tracked channel has
- **THEN** the daemon responds with HTTP status 400 naming that channel, and the members of "Chess" are unchanged

#### Scenario: Unknown category
- **WHEN** a client sets the members of a category identifier that does not exist
- **THEN** the daemon responds with HTTP status 404

### Requirement: Deleted Sources Leave Their Categories
When a tracked channel or playlist is deleted, the system SHALL remove it from every category it is a member of. The categories themselves SHALL remain, even when left with no members. Until the removal has been applied, a deleted source SHALL NOT contribute videos, counts or listed members to any category.

#### Scenario: Deleted channel leaves its categories
- **WHEN** the channel "@Hikaru", a member of "Chess" and "Streams", is deleted
- **THEN** neither "Chess" nor "Streams" lists "@Hikaru" as a member, and both categories still exist

#### Scenario: Category emptied by a deletion
- **WHEN** the only member of "Cooking" is deleted
- **THEN** "Cooking" is still listed, with no members and an unwatched count of 0
