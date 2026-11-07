## MODIFIED Requirements

### Requirement: Create Category
The system SHALL provide an HTTP endpoint that creates a category from a name and an optional color, AI prompt, list of channel handles and list of playlist IDs. A new category SHALL have a system-generated identifier, and no members unless given. The category and its members SHALL be stored together: a rejected request SHALL create nothing. The endpoint SHALL respond with HTTP status 201 and the created category: its identifier, its name, its color, its AI prompt and its member lists. A source repeated in a list SHALL be a member once. Every given source SHALL be tracked; otherwise the endpoint SHALL respond with HTTP status 400 naming the first untracked source and create nothing. Stored videos of the new members SHALL have their categories recomputed by the time the request completes. When another category already has the same name, the endpoint SHALL respond with HTTP status 409 and create nothing.

#### Scenario: Category created
- **WHEN** a client creates a category named "Chess" with the color "#d3ecf8" and no category has that name
- **THEN** the daemon responds with HTTP status 201 and a category named "Chess" with the color "#d3ecf8", an identifier and no members

#### Scenario: Category created without a color
- **WHEN** a client creates a category named "Chess" without a color
- **THEN** the daemon responds with HTTP status 201 and a category named "Chess" with its default color

#### Scenario: Category created with a prompt and sources
- **WHEN** a client creates "Chess" with the AI prompt "Chess games", the channel "@Hikaru" and the playlist "PLendgames", both tracked
- **THEN** the daemon responds with HTTP status 201 and "Chess" with that prompt and those two members, and the stored videos of both sources belong to "Chess"

#### Scenario: Untracked source on creation
- **WHEN** a client creates "Chess" with a channel handle no tracked channel has
- **THEN** the daemon responds with HTTP status 400 naming that channel and no category is created

#### Scenario: Prompt too long on creation
- **WHEN** a client creates "Chess" with an `ai_prompt` of 2001 characters
- **THEN** the daemon responds with HTTP status 400 and the message `Category AI prompt must be at most 2000 characters (got 2001)`, and no category is created

#### Scenario: Name already taken
- **WHEN** a client creates a category named "chess" and a category named "Chess" exists
- **THEN** the daemon responds with HTTP status 409 and no category is created

#### Scenario: Missing name
- **WHEN** a client creates a category without a name
- **THEN** the daemon responds with HTTP status 400 and no category is created

### Requirement: Category AI Prompt
A category SHALL have an optional AI prompt describing the videos that belong in it, none when created without one and for every category stored before this existed. The create and update endpoints SHALL take it as `ai_prompt`, trimmed; on update blank text SHALL remove it, and on create it SHALL leave the category without one. A prompt over 2000 characters SHALL be rejected with HTTP status 400 and the message `Category AI prompt must be at most 2000 characters (got <length>)`, changing nothing. It SHALL be accepted and kept whatever the preferences say, so turning AI or categorization off loses no prompt. It SHALL have no effect on a category's videos yet.

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
