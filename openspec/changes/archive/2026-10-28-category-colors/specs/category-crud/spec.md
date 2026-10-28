## ADDED Requirements

### Requirement: Category Color
Every category SHALL have a color: a `#` followed by 6 hexadecimal digits, stored and returned lowercase. Any other value SHALL be rejected with HTTP status 400 and nothing recorded. The palette SHALL be, in order: `#fff1bf`, `#ffe0c7`, `#ffd9d4`, `#fadceb`, `#e9def8`, `#dbe3fb`, `#d3ecf8`, `#cff0ea`, `#e4f0cc`, `#eee5d7`. Any valid color SHALL be accepted, whether or not it is in the palette.

#### Scenario: Color normalised to lowercase
- **WHEN** a category is created with the color "#D3ECF8"
- **THEN** its color is "#d3ecf8"

#### Scenario: Custom color
- **WHEN** a category is created with the color "#f0e4d0", which is not in the palette
- **THEN** its color is "#f0e4d0"

#### Scenario: Invalid color
- **WHEN** a category is created or updated with the color "yellow", "#fff" or "#gggggg"
- **THEN** the daemon responds with HTTP status 400 and records nothing

### Requirement: Default Category Color
A category created without a color SHALL get the palette color used by the fewest categories, the earliest in palette order on a tie.

#### Scenario: First category
- **WHEN** a category is created without a color and no category exists
- **THEN** its color is "#fff1bf"

#### Scenario: Next unused color
- **WHEN** a category is created without a color and the existing categories use "#fff1bf" and "#ffd9d4"
- **THEN** its color is "#ffe0c7"

#### Scenario: Every palette color used
- **WHEN** a category is created without a color, every palette color is used once and "#fff1bf" is used twice
- **THEN** its color is "#ffe0c7"

### Requirement: Existing Categories Get Colors
On upgrade, every category that existed before colors were introduced SHALL be given a palette color, assigned in palette order by creation time and wrapping after the tenth.

#### Scenario: Upgrading with categories
- **WHEN** the daemon upgrades a database holding the categories "Chess", "Science" and "Sports", created in that order
- **THEN** their colors are "#fff1bf", "#ffe0c7" and "#ffd9d4"

## MODIFIED Requirements

### Requirement: Create Category
The system SHALL provide an HTTP endpoint that creates a category from a name and an optional color. A new category SHALL have no members and a system-generated identifier. The endpoint SHALL respond with HTTP status 201 and the created category: its identifier, its name, its color and its (empty) member lists. When another category already has the same name, the endpoint SHALL respond with HTTP status 409 and create nothing.

#### Scenario: Category created
- **WHEN** a client creates a category named "Chess" with the color "#d3ecf8" and no category has that name
- **THEN** the daemon responds with HTTP status 201 and a category named "Chess" with the color "#d3ecf8", an identifier and no members

#### Scenario: Category created without a color
- **WHEN** a client creates a category named "Chess" without a color
- **THEN** the daemon responds with HTTP status 201 and a category named "Chess" with its default color

#### Scenario: Name already taken
- **WHEN** a client creates a category named "chess" and a category named "Chess" exists
- **THEN** the daemon responds with HTTP status 409 and no category is created

#### Scenario: Missing name
- **WHEN** a client creates a category without a name
- **THEN** the daemon responds with HTTP status 400 and no category is created

### Requirement: List Categories
The system SHALL provide an HTTP endpoint that returns every category, ordered by name ignoring case, together with the unwatched total across all categories. Each category SHALL include its identifier, its name, its color, its unwatched count, the handles of its channel members and the IDs of its playlist members. The unwatched total SHALL be the number of distinct YouTube videos counted by any category, each category counting as stated in Category Unwatched Count.

#### Scenario: Categories listed by name
- **WHEN** the categories "science", "Chess" and "Sports" exist
- **THEN** the daemon responds with HTTP status 200 and lists them in the order "Chess", "science", "Sports"

#### Scenario: Color listed
- **WHEN** the category "Chess" has the color "#d3ecf8"
- **THEN** its listed color is "#d3ecf8"

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

### Requirement: Rename Category
The system SHALL provide an HTTP endpoint that, given a category's identifier and a name, a color or both, renames and/or recolors the category and responds with HTTP status 204. A request with neither SHALL be rejected with HTTP status 400. Updating SHALL keep the category's identifier and members. When another category already has the same name, the endpoint SHALL respond with HTTP status 409 and change nothing, color included. Renaming a category to a name differing from its own only in case SHALL be allowed. When no category has the identifier, the endpoint SHALL respond with HTTP status 404.

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
- **WHEN** a client updates a category with neither a name nor a color
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
