## MODIFIED Requirements

### Requirement: Categorization Settings
`categorization_model` SHALL be a non-blank name of at most 200 characters, else rejected with `Categorization model must be named (got "<value>")`. `categorization_context_length` SHALL be an integer from 1024 to 131072, else rejected with `Context length must be between 1024 and 131072 (got <value>)`. `categorization_temperature` SHALL be a number from 0 to 2, else rejected with `Temperature must be between 0 and 2 (got <value>)`. `categorization_prompt` SHALL be free text that is not blank, else rejected with `Categorization prompt must not be empty`; it has no sections or variables. Each rejection is HTTP status 400. Changing a categorization setting SHALL NOT affect summaries.

#### Scenario: Saving categorization settings
- **WHEN** a client sends `PATCH /api/preferences` with `categorization_enabled` `true`, `categorization_model` `"gemma3:4b"`, `categorization_context_length` `8192`, `categorization_temperature` `0.5`, `categorization_thinking` `false` and `categorization_prompt` `"Answer yes or no."`, AI being on
- **THEN** the daemon responds with HTTP status 200 and those values, with `categorization_prompt_is_default` `false`

#### Scenario: Blank model
- **WHEN** a client sends `PATCH /api/preferences` with `categorization_model` `"  "`
- **THEN** the daemon responds with HTTP status 400 and the message `Categorization model must be named (got "  ")`

#### Scenario: Context length out of range
- **WHEN** a client sends `PATCH /api/preferences` with `categorization_context_length` `512`
- **THEN** the daemon responds with HTTP status 400 and the message `Context length must be between 1024 and 131072 (got 512)`

#### Scenario: Temperature out of range
- **WHEN** a client sends `PATCH /api/preferences` with `categorization_temperature` `2.5`
- **THEN** the daemon responds with HTTP status 400 and the message `Temperature must be between 0 and 2 (got 2.5)`

#### Scenario: Blank prompt
- **WHEN** a client sends `PATCH /api/preferences` with `categorization_prompt` `"  "`
- **THEN** the daemon responds with HTTP status 400 and the message `Categorization prompt must not be empty`

#### Scenario: Summaries left alone
- **WHEN** a video's summary is Failed and a client changes `categorization_model`
- **THEN** the video's summary stays Failed
