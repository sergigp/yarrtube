## MODIFIED Requirements

### Requirement: Preferences And Their Defaults
The daemon SHALL keep one set of preferences: `categories_enabled`, `home_show_continue_watching`, `home_show_categories`, `home_show_quick_watches` and `home_show_latest`, each a boolean that defaults to `true`; `theme`, either `"light"` or `"dark"`, that defaults to `"light"`; `ai_enabled`, `summaries_enabled` and `categorization_enabled`, booleans that default to `false`; `ollama_url`, an http(s) URL that defaults to unset (`null`); `summaries_model`, defaulting to `"qwen3:4b"`; `summaries_context_length`, defaulting to `12288`; `summaries_temperature`, defaulting to `0.3`; `summaries_thinking`, defaulting to `true`; and `summaries_prompt`, defaulting to the summary prompt shipped with the running version (`prompts/summaries/v3.md`), read alongside `summaries_prompt_is_default`; `categorization_model`, defaulting to `"qwen3:4b"`; `categorization_context_length`, defaulting to `4096`; `categorization_temperature`, defaulting to `0`; `categorization_thinking`, defaulting to `true`; and `categorization_prompt`, defaulting to the categorization prompt shipped with the running version (`prompts/categorization/v2.md`), read alongside `categorization_prompt_is_default`. A preference never saved SHALL read as its default, including after an upgrade that adds a new preference. A stored `theme` that is neither `"light"` nor `"dark"` SHALL read as `"light"`.

#### Scenario: Fresh install
- **WHEN** no preferences have been saved
- **THEN** every display boolean preference reads as `true`, `theme` reads as `"light"`, `ai_enabled`, `summaries_enabled` and `categorization_enabled` read as `false`, `ollama_url` reads as `null`, the summary settings read as `qwen3:4b`, `12288`, `0.3`, thinking on and the shipped prompt, and the categorization settings read as `qwen3:4b`, `4096`, `0`, thinking on and the shipped prompt

#### Scenario: Preferences survive a restart
- **WHEN** `categories_enabled` is saved as `false` and the daemon restarts
- **THEN** `categories_enabled` reads as `false`

#### Scenario: Upgrading from a version without a theme
- **WHEN** preferences were saved by a version that had no `theme` preference
- **THEN** the saved preferences keep their values and `theme` reads as `"light"`

#### Scenario: Upgrading from a version without AI preferences
- **WHEN** preferences were saved by a version that had no AI preferences
- **THEN** the saved preferences keep their values and every AI, summary and categorization preference reads as its default

#### Scenario: Upgrading from a version without the categorization generation settings
- **WHEN** preferences were saved by a version that had `categorization_model` but no context length, temperature or thinking for categorization
- **THEN** the saved preferences keep their values and those three read as `4096`, `0` and `true`, not the summary defaults

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
- **WHEN** a video's summary is unavailable and a client changes `categorization_model`
- **THEN** the video's summary stays unavailable
