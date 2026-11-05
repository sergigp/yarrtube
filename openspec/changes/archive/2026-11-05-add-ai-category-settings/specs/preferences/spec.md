## MODIFIED Requirements

### Requirement: Preferences And Their Defaults
The daemon SHALL keep one set of preferences: `categories_enabled`, `home_show_continue_watching`, `home_show_categories`, `home_show_quick_watches` and `home_show_latest`, each a boolean that defaults to `true`; `theme`, either `"light"` or `"dark"`, that defaults to `"light"`; `ai_enabled`, `summaries_enabled` and `categorization_enabled`, booleans that default to `false`; `ollama_url`, an http(s) URL that defaults to unset (`null`); `summaries_model`, defaulting to `"qwen3:4b"`; `summaries_context_length`, defaulting to `12288`; `summaries_temperature`, defaulting to `0.3`; `summaries_thinking`, defaulting to `true`; and `summaries_prompt`, defaulting to the summary prompt shipped with the running version (`prompts/summaries/v3.md`), read alongside `summaries_prompt_is_default`; `categorization_model`, defaulting to `"qwen3:4b"`; and `categorization_prompt`, defaulting to the categorization prompt shipped with the running version (`prompts/categorization/v1.md`), read alongside `categorization_prompt_is_default`. A preference never saved SHALL read as its default, including after an upgrade that adds a new preference. A stored `theme` that is neither `"light"` nor `"dark"` SHALL read as `"light"`.

#### Scenario: Fresh install
- **WHEN** no preferences have been saved
- **THEN** every display boolean preference reads as `true`, `theme` reads as `"light"`, `ai_enabled`, `summaries_enabled` and `categorization_enabled` read as `false`, `ollama_url` reads as `null`, the summary settings read as `qwen3:4b`, `12288`, `0.3`, thinking on and the shipped prompt, and the categorization settings read as `qwen3:4b` and the shipped prompt

#### Scenario: Preferences survive a restart
- **WHEN** `categories_enabled` is saved as `false` and the daemon restarts
- **THEN** `categories_enabled` reads as `false`

#### Scenario: Upgrading from a version without a theme
- **WHEN** preferences were saved by a version that had no `theme` preference
- **THEN** the saved preferences keep their values and `theme` reads as `"light"`

#### Scenario: Upgrading from a version without AI preferences
- **WHEN** preferences were saved by a version that had no AI preferences
- **THEN** the saved preferences keep their values and every AI, summary and categorization preference reads as its default

## ADDED Requirements

### Requirement: Saved Or Default Categorization Prompt
Until a `categorization_prompt` is saved, it SHALL read as the prompt shipped with the running version, with `categorization_prompt_is_default` `true`. Saving `categorization_prompt` SHALL store that text, kept as is across upgrades, with `categorization_prompt_is_default` `false`; saving other preferences SHALL NOT store it. Saving `categorization_prompt` as `null` SHALL drop the saved prompt, so it reads as the shipped one again.

#### Scenario: Never saved
- **WHEN** other preferences have been saved but `categorization_prompt` never has
- **THEN** `categorization_prompt` reads as the shipped prompt and `categorization_prompt_is_default` as `true`

#### Scenario: Saved prompt survives an upgrade
- **WHEN** a client saves a `categorization_prompt` and the daemon is upgraded to a version shipping a different prompt
- **THEN** `categorization_prompt` reads as the saved text and `categorization_prompt_is_default` as `false`

#### Scenario: Back to the shipped prompt
- **WHEN** a prompt is saved and a client sends `PATCH /api/preferences` with `categorization_prompt` `null`
- **THEN** the daemon responds with HTTP status 200, the shipped prompt and `categorization_prompt_is_default` `true`

### Requirement: Categorization Settings
`categorization_model` SHALL be a non-blank name of at most 200 characters, else rejected with `Categorization model must be named (got "<value>")`. `categorization_prompt` SHALL be free text that is not blank, else rejected with `Categorization prompt must not be empty`; it has no sections or variables. Each rejection is HTTP status 400. Changing a categorization setting SHALL NOT affect summaries.

#### Scenario: Saving categorization settings
- **WHEN** a client sends `PATCH /api/preferences` with `categorization_enabled` `true`, `categorization_model` `"gemma3:4b"` and `categorization_prompt` `"Answer yes or no."`, AI being on
- **THEN** the daemon responds with HTTP status 200 and those values, with `categorization_prompt_is_default` `false`

#### Scenario: Blank model
- **WHEN** a client sends `PATCH /api/preferences` with `categorization_model` `"  "`
- **THEN** the daemon responds with HTTP status 400 and the message `Categorization model must be named (got "  ")`

#### Scenario: Blank prompt
- **WHEN** a client sends `PATCH /api/preferences` with `categorization_prompt` `"  "`
- **THEN** the daemon responds with HTTP status 400 and the message `Categorization prompt must not be empty`

#### Scenario: Summaries left alone
- **WHEN** a video's summary is unavailable and a client changes `categorization_model`
- **THEN** the video's summary stays unavailable
