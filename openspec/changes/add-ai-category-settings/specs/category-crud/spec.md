## MODIFIED Requirements

### Requirement: Rename Category
The system SHALL provide an HTTP endpoint that, given a category's identifier and any of a name, a color and an AI prompt, applies those to the category and responds with HTTP status 204. A request with none of them SHALL be rejected with HTTP status 400. Updating SHALL keep the category's identifier and members. When another category already has the same name, the endpoint SHALL respond with HTTP status 409 and change nothing, color included. Renaming a category to a name differing from its own only in case SHALL be allowed. When no category has the identifier, the endpoint SHALL respond with HTTP status 404.

#### Scenario: Category renamed
- **WHEN** a client renames the category "Chess" to "Board games"
- **THEN** the daemon responds with HTTP status 204 and the category, with the same identifier, color and members, is named "Board games"

#### Scenario: Category recolored
- **WHEN** a client sets the color of the category "Chess" to "#e9def8" without a name
- **THEN** the daemon responds with HTTP status 204 and "Chess", with the same name and members, has the color "#e9def8"

#### Scenario: Renamed and recolored together
- **WHEN** a client updates "Chess" with the name "Board games" and the color "#e9def8"
- **THEN** the daemon responds with HTTP status 204 and the category is named "Board games" with the color "#e9def8"

#### Scenario: Neither name nor color
- **WHEN** a client updates a category with no name, color or AI prompt
- **THEN** the daemon responds with HTTP status 400 and the category is unchanged

#### Scenario: Changing only the case
- **WHEN** a client renames the category "chess" to "Chess"
- **THEN** the daemon responds with HTTP status 204 and the category is named "Chess"

#### Scenario: Name taken by another category
- **WHEN** a client updates "Chess" with the name "sports" and the color "#e9def8", and a category named "Sports" exists
- **THEN** the daemon responds with HTTP status 409 and both categories are unchanged, color included

#### Scenario: Unknown category
- **WHEN** a client updates a category identifier that does not exist
- **THEN** the daemon responds with HTTP status 404

### Requirement: List Categories
The system SHALL provide an HTTP endpoint that returns every category, ordered by name ignoring case, together with the unwatched total across all categories. Each category SHALL include its identifier, its name, its color, its AI prompt (`ai_prompt`, `null` when it has none), its unwatched count, the handles of its channel members and the IDs of its playlist members. The unwatched total SHALL be the number of distinct YouTube videos counted by any category, each category counting as stated in Category Unwatched Count.

#### Scenario: Categories listed by name
- **WHEN** the categories "science", "Chess" and "Sports" exist
- **THEN** the daemon responds with HTTP status 200 and lists them in the order "Chess", "science", "Sports"

#### Scenario: Color listed
- **WHEN** the category "Chess" has the color "#d3ecf8"
- **THEN** its listed color is "#d3ecf8"

#### Scenario: AI prompt listed
- **WHEN** the category "Chess" has the AI prompt "Chess games and openings" and "Science" has none
- **THEN** the listed `ai_prompt` of "Chess" is "Chess games and openings" and that of "Science" is `null`

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

## ADDED Requirements

### Requirement: Category AI Prompt
A category SHALL have an optional AI prompt describing the videos that belong in it, none when created and for every category stored before this existed. The update endpoint SHALL take it as `ai_prompt`, trimmed; blank text SHALL remove it. A prompt over 2000 characters SHALL be rejected with HTTP status 400 and the message `Category AI prompt must be at most 2000 characters (got <length>)`, changing nothing. It SHALL be accepted and kept whatever the preferences say, so turning AI or categorization off loses no prompt. It SHALL have no effect on a category's videos yet.

#### Scenario: Setting the prompt
- **WHEN** a client updates "Chess" with `ai_prompt` "  Chess games and openings  "
- **THEN** the daemon responds with HTTP status 204 and "Chess", with the same name, color and members, has the AI prompt "Chess games and openings"

#### Scenario: Removing the prompt
- **WHEN** "Chess" has an AI prompt and a client updates it with `ai_prompt` ""
- **THEN** the daemon responds with HTTP status 204 and "Chess" has no AI prompt

#### Scenario: Other updates keep the prompt
- **WHEN** "Chess" has an AI prompt and a client renames it
- **THEN** "Chess" keeps its AI prompt

#### Scenario: Prompt too long
- **WHEN** a client updates "Chess" with an `ai_prompt` of 2001 characters
- **THEN** the daemon responds with HTTP status 400 and the message `Category AI prompt must be at most 2000 characters (got 2001)`, and "Chess" is unchanged

#### Scenario: AI off
- **WHEN** AI is off and a client updates "Chess" with an `ai_prompt`
- **THEN** the daemon responds with HTTP status 204 and stores it

#### Scenario: Existing categories after upgrade
- **WHEN** the daemon starts on a database with categories stored before AI prompts existed
- **THEN** every one of them has no AI prompt
