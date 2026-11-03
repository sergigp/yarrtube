# preferences Specification

## Purpose
Holds the user's daemon-wide preferences: display choices (whether categories are used, which home sections show, the theme) and AI settings (the Ollama server, and how video summaries are generated, including their prompt), persisted by the daemon so every browser and device sees the same choices.

## Requirements

### Requirement: Preferences And Their Defaults
The daemon SHALL keep one set of preferences: `categories_enabled`, `home_show_continue_watching`, `home_show_categories`, `home_show_quick_watches` and `home_show_latest`, each a boolean that defaults to `true`; `theme`, either `"light"` or `"dark"`, that defaults to `"light"`; `ai_enabled` and `summaries_enabled`, booleans that default to `false`; `ollama_url`, an http(s) URL that defaults to unset (`null`); `summaries_model`, defaulting to `"qwen3:4b"`; `summaries_context_length`, defaulting to `12288`; `summaries_temperature`, defaulting to `0.3`; `summaries_thinking`, defaulting to `true`; and `summaries_prompt`, defaulting to the summary prompt shipped with the running version (`prompts/summaries/v3.md`), read alongside `summaries_prompt_is_default`. A preference never saved SHALL read as its default, including after an upgrade that adds a new preference. A stored `theme` that is neither `"light"` nor `"dark"` SHALL read as `"light"`.

#### Scenario: Fresh install
- **WHEN** no preferences have been saved
- **THEN** every display boolean preference reads as `true`, `theme` reads as `"light"`, `ai_enabled` and `summaries_enabled` read as `false`, `ollama_url` reads as `null`, and the summary settings read as `qwen3:4b`, `12288`, `0.3`, thinking on and the shipped prompt

#### Scenario: Preferences survive a restart
- **WHEN** `categories_enabled` is saved as `false` and the daemon restarts
- **THEN** `categories_enabled` reads as `false`

#### Scenario: Upgrading from a version without a theme
- **WHEN** preferences were saved by a version that had no `theme` preference
- **THEN** the saved preferences keep their values and `theme` reads as `"light"`

#### Scenario: Upgrading from a version without AI preferences
- **WHEN** preferences were saved by a version that had no AI preferences
- **THEN** the saved preferences keep their values and every AI and summary preference reads as its default

### Requirement: Read Preferences
The daemon SHALL provide `GET /api/preferences`, responding with HTTP status 200 and every preference with its current value.

#### Scenario: Reading the defaults
- **WHEN** a client requests `GET /api/preferences` and nothing has been saved
- **THEN** the daemon responds with HTTP status 200 and every preference set to its default

### Requirement: Save Preferences
The daemon SHALL provide `PATCH /api/preferences` taking any subset of the preferences. On success it SHALL store the sent preferences, keep every preference not sent, and respond with HTTP status 200 and every preference. A request with any invalid value SHALL be rejected with HTTP status 400 and change nothing, including the valid values sent with it; a malformed body (e.g. a non-boolean value) is rejected as by every other endpoint. A `theme` other than `"light"` or `"dark"` SHALL be rejected with the message `Theme must be "light" or "dark" (got "<value>")`. `PUT /api/preferences` SHALL no longer be served.

#### Scenario: Saving preferences
- **WHEN** a client sends `PATCH /api/preferences` with `categories_enabled` `false`
- **THEN** the daemon responds with HTTP status 200 and every preference, `categories_enabled` `false`, and a later `GET /api/preferences` returns them

#### Scenario: Missing preference
- **WHEN** `home_show_latest` is saved as `false` and a client sends `PATCH /api/preferences` with only `categories_enabled` `false`
- **THEN** the daemon responds with HTTP status 200, `categories_enabled` `false`, `home_show_latest` still `false` and every other preference unchanged, and a later `GET /api/preferences` returns the same

#### Scenario: Saving the dark theme
- **WHEN** a client sends `PATCH /api/preferences` with only `theme` `"dark"`
- **THEN** the daemon responds with HTTP status 200 and `theme` `"dark"`, and a later `GET /api/preferences` returns `theme` `"dark"`

#### Scenario: Missing theme
- **WHEN** `theme` is saved as `"dark"` and a client sends `PATCH /api/preferences` without `theme`
- **THEN** the daemon responds with HTTP status 200 and `theme` is still `"dark"`

#### Scenario: Empty patch
- **WHEN** a client sends `PATCH /api/preferences` with `{}`
- **THEN** the daemon responds with HTTP status 200 and the stored preferences, unchanged

#### Scenario: Unknown theme
- **WHEN** a client sends `PATCH /api/preferences` with `theme` `"sepia"`
- **THEN** the daemon responds with HTTP status 400 and the message `Theme must be "light" or "dark" (got "sepia")`, and the stored preferences are unchanged

#### Scenario: One invalid value among valid ones
- **WHEN** a client sends `PATCH /api/preferences` with `categories_enabled` `false` and `summaries_temperature` `5`
- **THEN** the daemon responds with HTTP status 400 and `categories_enabled` is unchanged

### Requirement: Category Data Kept While Categories Are Off
Turning `categories_enabled` off SHALL NOT change any category, category membership, video category or Plex category label, and the category endpoints SHALL keep working, so turning it back on shows everything as before.

#### Scenario: Turning categories back on
- **WHEN** "Chess" has 2 sources, categories are turned off and then on again
- **THEN** "Chess" still exists with its 2 sources and its videos still belong to it

### Requirement: Saved Or Default Summary Prompt
Until a `summaries_prompt` is saved, it SHALL read as the prompt shipped with the running version, with `summaries_prompt_is_default` `true`. Saving `summaries_prompt` SHALL store that text, kept as is across upgrades, with `summaries_prompt_is_default` `false`; saving other preferences SHALL NOT store it. Saving `summaries_prompt` as `null` SHALL drop the saved prompt, so it reads as the shipped one again.

#### Scenario: Never saved
- **WHEN** other preferences have been saved but `summaries_prompt` never has
- **THEN** `summaries_prompt` reads as the shipped prompt and `summaries_prompt_is_default` as `true`

#### Scenario: Shipped prompt changes before the user saves one
- **WHEN** `summaries_prompt` was never saved and the daemon is upgraded to a version shipping a different prompt
- **THEN** `summaries_prompt` reads as the new shipped prompt

#### Scenario: Saved prompt survives an upgrade
- **WHEN** a client saves a `summaries_prompt` and the daemon is upgraded to a version shipping a different prompt
- **THEN** `summaries_prompt` reads as the saved text and `summaries_prompt_is_default` as `false`

#### Scenario: Back to the shipped prompt
- **WHEN** a prompt is saved and a client sends `PATCH /api/preferences` with `summaries_prompt` `null`
- **THEN** the daemon responds with HTTP status 200, the shipped prompt and `summaries_prompt_is_default` `true`

### Requirement: Ollama URL Preference
`ollama_url` SHALL be saved from an absolute `http` or `https` URL, stored without a trailing slash; an empty string SHALL unset it. Any other value SHALL be rejected with HTTP status 400 and the message `Ollama URL must be an http(s) URL (got "<value>")`. Saving does not contact the server.

#### Scenario: Saving a URL
- **WHEN** a client sends `PATCH /api/preferences` with `ollama_url` `"http://mac.local:11434/"`
- **THEN** the daemon responds with HTTP status 200 and `ollama_url` `"http://mac.local:11434"`

#### Scenario: Clearing the URL
- **WHEN** AI is off and a client sends `PATCH /api/preferences` with `ollama_url` `""`
- **THEN** the daemon responds with HTTP status 200 and `ollama_url` `null`

#### Scenario: Invalid URL
- **WHEN** a client sends `PATCH /api/preferences` with `ollama_url` `"mac.local"`
- **THEN** the daemon responds with HTTP status 400 and the message `Ollama URL must be an http(s) URL (got "mac.local")`

### Requirement: AI Needs An Ollama URL
A save that would leave `ai_enabled` `true` with `ollama_url` unset SHALL be rejected with HTTP status 400 and the message `Ollama URL must be set to enable AI`.

#### Scenario: Enabling AI without a URL
- **WHEN** no Ollama URL is saved and a client sends `PATCH /api/preferences` with `ai_enabled` `true`
- **THEN** the daemon responds with HTTP status 400 and the message `Ollama URL must be set to enable AI`

#### Scenario: Enabling AI with its URL
- **WHEN** a client sends `PATCH /api/preferences` with `ai_enabled` `true` and `ollama_url` `"http://mac.local:11434"`
- **THEN** the daemon responds with HTTP status 200 and both saved

#### Scenario: Clearing the URL while AI is on
- **WHEN** AI is on and a client sends `PATCH /api/preferences` with `ollama_url` `""`
- **THEN** the daemon responds with HTTP status 400 and the message `Ollama URL must be set to enable AI`

### Requirement: Summary Generation Settings
`summaries_model` SHALL be a non-blank name of at most 200 characters, else rejected with `Summary model must be named (got "<value>")`. `summaries_context_length` SHALL be an integer from 1024 to 131072, else rejected with `Context length must be between 1024 and 131072 (got <value>)`. `summaries_temperature` SHALL be a number from 0 to 2, else rejected with `Temperature must be between 0 and 2 (got <value>)`. Each rejection is HTTP status 400.

#### Scenario: Saving summary settings
- **WHEN** a client sends `PATCH /api/preferences` with `summaries_model` `"gemma3:4b"`, `summaries_context_length` `8192`, `summaries_temperature` `0` and `summaries_thinking` `false`
- **THEN** the daemon responds with HTTP status 200 and those values

#### Scenario: Blank model
- **WHEN** a client sends `PATCH /api/preferences` with `summaries_model` `"  "`
- **THEN** the daemon responds with HTTP status 400 and the message `Summary model must be named (got "  ")`

#### Scenario: Context length out of range
- **WHEN** a client sends `PATCH /api/preferences` with `summaries_context_length` `512`
- **THEN** the daemon responds with HTTP status 400 and the message `Context length must be between 1024 and 131072 (got 512)`

#### Scenario: Temperature out of range
- **WHEN** a client sends `PATCH /api/preferences` with `summaries_temperature` `2.5`
- **THEN** the daemon responds with HTTP status 400 and the message `Temperature must be between 0 and 2 (got 2.5)`

### Requirement: Summary Prompt Sections
`summaries_prompt` SHALL be a document of sections, each starting at a `## <name>` line; text before the first section is a free-form note. It SHALL hold exactly once each of `system`, `user`, `chapters`, `map system`, `map user`, `map chapters`, `reduce system`, `reduce user` and `reduce part`, each with non-blank text, and no other section. Otherwise the save SHALL be rejected with HTTP status 400 and the message for the first problem found.

#### Scenario: Saving an edited prompt
- **WHEN** a client sends `PATCH /api/preferences` with the shipped prompt with one rule added to `system`
- **THEN** the daemon responds with HTTP status 200 and that prompt, unchanged

#### Scenario: Missing section
- **WHEN** a client sends a `summaries_prompt` without a `## map user` section
- **THEN** the daemon responds with HTTP status 400 and the message `Prompt is missing the "map user" section`

#### Scenario: Unknown section
- **WHEN** a client sends a `summaries_prompt` with an extra `## sytem` section
- **THEN** the daemon responds with HTTP status 400 and the message `Prompt has an unknown section "sytem"`

#### Scenario: Repeated section
- **WHEN** a client sends a `summaries_prompt` with two `## user` sections
- **THEN** the daemon responds with HTTP status 400 and the message `Prompt has the "user" section twice`

#### Scenario: Empty section
- **WHEN** a client sends a `summaries_prompt` whose `## chapters` section has only blank lines
- **THEN** the daemon responds with HTTP status 400 and the message `Prompt section "chapters" is empty`

### Requirement: Summary Prompt Variables
A `{name}` (lowercase letters and `_`) in a section SHALL be a variable, and each section SHALL allow only its own variables; any other SHALL be rejected with HTTP status 400. Braces not forming a variable are plain text.

#### Scenario: Allowed variables per section
- **WHEN** a client sends a `summaries_prompt` whose sections use only their own variables: `system`, `map system`, `reduce system`: `lang`; `user`: `title`, `minutes`, `chapters`, `transcript`, `lang`; `chapters`, `map chapters`: `chapter_list`; `map user`: `title`, `minutes`, `part`, `parts`, `span`, `chapters`, `transcript`, `lang`; `reduce user`: `title`, `minutes`, `chapters`, `part_summaries`, `lang`; `reduce part`: `part`, `span`, `summary`
- **THEN** the daemon responds with HTTP status 200

#### Scenario: Variable not allowed in its section
- **WHEN** a client sends a `summaries_prompt` whose `reduce system` section uses `{transcript}`
- **THEN** the daemon responds with HTTP status 400 and the message `Prompt section "reduce system" can't use {transcript}; it can use {lang}`

#### Scenario: Note before the first section
- **WHEN** a client sends a valid `summaries_prompt` whose text before the first section mentions `{name}`
- **THEN** the daemon responds with HTTP status 200

#### Scenario: Braces that are not a variable
- **WHEN** a client sends a valid `summaries_prompt` whose `system` section contains `{"match": true}`
- **THEN** the daemon responds with HTTP status 200
