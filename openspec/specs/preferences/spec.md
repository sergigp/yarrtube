# preferences Specification

## Purpose
Holds the user's daemon-wide preferences: display choices (whether categories are used, which home sections show, the theme) and AI settings (the Ollama server, and how video summaries are generated, including their prompt), persisted by the daemon so every browser and device sees the same choices.

## Requirements

### Requirement: Preferences And Their Defaults
The daemon SHALL keep one set of preferences: `categories_enabled`, `home_show_continue_watching`, `home_show_categories`, `home_show_quick_watches` and `home_show_latest`, each a boolean that defaults to `true`; `theme`, either `"light"` or `"dark"`, that defaults to `"light"`; `ai_enabled`, `summaries_enabled` and `categorization_enabled`, booleans that default to `false`; `ai_provider`, either `"ollama"` or `"anthropic"`, that defaults to `"ollama"`; `ollama_url`, an http(s) URL that defaults to unset (`null`); `summaries_model`, defaulting to `"qwen3:4b"`; `summaries_context_length`, defaulting to `12288`; `summaries_temperature`, defaulting to `0.3`; `summaries_thinking`, defaulting to `true`; and `summaries_prompt`, defaulting to the summary prompt shipped with the running version (`prompts/summaries/v3.md`), read alongside `summaries_prompt_is_default`; `categorization_model`, defaulting to `"qwen3:4b"`; `categorization_context_length`, defaulting to Auto (see "Auto Categorization Context Length"), read alongside `categorization_context_length_is_auto`; `categorization_temperature`, defaulting to `0`; `categorization_thinking`, defaulting to `true`; and `categorization_prompt`, defaulting to the categorization prompt shipped with the running version (`prompts/categorization/v2-batch.md`), read alongside `categorization_prompt_is_default`; `anthropic_summaries_model` and `anthropic_categorization_model`, defaulting to `"claude-haiku-5-5"`; and `anthropic_summaries_effort` and `anthropic_categorization_effort`, defaulting to `"low"`. Every preference SHALL be read alongside `anthropic_available` (see "Anthropic Availability"). A preference never saved SHALL read as its default, including after an upgrade that adds a new preference. A stored `theme` that is neither `"light"` nor `"dark"` SHALL read as `"light"`.

#### Scenario: Fresh install
- **WHEN** no preferences have been saved and no category has an AI prompt
- **THEN** every display boolean preference reads as `true`, `theme` reads as `"light"`, `ai_enabled`, `summaries_enabled` and `categorization_enabled` read as `false`, `ai_provider` reads as `"ollama"`, `ollama_url` reads as `null`, the summary settings read as `qwen3:4b`, `12288`, `0.3`, thinking on and the shipped prompt, the categorization settings read as `qwen3:4b`, `4096` with `categorization_context_length_is_auto` `true`, `0`, thinking on and the shipped prompt, and both Anthropic models read as `claude-haiku-5-5` with effort `low`

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
- **THEN** the saved preferences keep their values, the context length reads as Auto, and temperature and thinking read as `0` and `true`, not the summary defaults

#### Scenario: Upgrading from a version without providers
- **WHEN** preferences were saved by a version with AI on and an Ollama URL but no `ai_provider`
- **THEN** the saved preferences keep their values, `ai_provider` reads as `"ollama"` and AI keeps running on Ollama

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
A save that would leave `ai_enabled` `true` with `ai_provider` `"ollama"` and `ollama_url` unset SHALL be rejected with HTTP status 400 and the message `Ollama URL must be set to enable AI`. With `ai_provider` `"anthropic"`, `ollama_url` MAY be unset.

#### Scenario: Enabling AI without a URL
- **WHEN** no Ollama URL is saved and a client sends `PATCH /api/preferences` with `ai_enabled` `true`
- **THEN** the daemon responds with HTTP status 400 and the message `Ollama URL must be set to enable AI`

#### Scenario: Enabling AI with its URL
- **WHEN** a client sends `PATCH /api/preferences` with `ai_enabled` `true` and `ollama_url` `"http://mac.local:11434"`
- **THEN** the daemon responds with HTTP status 200 and both saved

#### Scenario: Clearing the URL while AI is on
- **WHEN** AI is on with Ollama and a client sends `PATCH /api/preferences` with `ollama_url` `""`
- **THEN** the daemon responds with HTTP status 400 and the message `Ollama URL must be set to enable AI`

#### Scenario: Anthropic without a URL
- **WHEN** a key is set, no Ollama URL is saved and a client sends `PATCH /api/preferences` with `ai_enabled` `true` and `ai_provider` `"anthropic"`
- **THEN** the daemon responds with HTTP status 200 and both saved

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
`categorization_model` SHALL be a non-blank name of at most 200 characters, else rejected with `Categorization model must be named (got "<value>")`. `categorization_context_length` SHALL be an integer from 1024 to 131072, else rejected with `Context length must be between 1024 and 131072 (got <value>)`. `categorization_temperature` SHALL be a number from 0 to 2, else rejected with `Temperature must be between 0 and 2 (got <value>)`. `categorization_prompt` SHALL be free text that is not blank, else rejected with `Categorization prompt must not be empty`; it has no sections or variables. Each rejection is HTTP status 400. Changing a categorization setting SHALL NOT affect summaries. Changing any categorization setting other than `categorization_enabled` SHALL reset failed categorizations (see "Failed Verdicts Reset" in `video-categorization`).

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

#### Scenario: Thinking changed after a failure
- **WHEN** a video's categorization is Failed with a `failed` verdict and a client changes `categorization_thinking`
- **THEN** the video's categorization is Pending and the `failed` verdict is deleted

### Requirement: Auto Categorization Context Length
Until a `categorization_context_length` is saved, it SHALL be Auto: 1,820 tokens per category with an AI prompt, rounded up to a multiple of 1,024, at least 4,096 and at most 131,072, read as that number with `categorization_context_length_is_auto` `true`. Saving a number SHALL store it, with `_is_auto` `false`; saving `null` SHALL drop it, back to Auto. Saving other preferences SHALL NOT store it. A `4096` stored by a version before Auto, its old default, SHALL be dropped once by a migration and read as Auto; a `4096` saved since SHALL read as saved.

#### Scenario: Nine categories
- **WHEN** no context length is saved and 9 categories have an AI prompt
- **THEN** `categorization_context_length` reads as `16384` and `categorization_context_length_is_auto` as `true`

#### Scenario: Category prompt added
- **WHEN** no context length is saved, 9 categories have an AI prompt, and a 10th gets one
- **THEN** `categorization_context_length` reads as `18432`

#### Scenario: Few categories
- **WHEN** no context length is saved and 2 categories have an AI prompt
- **THEN** `categorization_context_length` reads as `4096`

#### Scenario: Override
- **WHEN** a client sends `PATCH /api/preferences` with `categorization_context_length` `8192`
- **THEN** it reads as `8192` with `categorization_context_length_is_auto` `false`, whatever the number of categories

#### Scenario: Back to Auto
- **WHEN** `8192` is saved and a client sends `PATCH /api/preferences` with `categorization_context_length` `null`
- **THEN** the daemon responds with HTTP status 200 and the Auto value, with `categorization_context_length_is_auto` `true`

#### Scenario: Other preference saved
- **WHEN** no context length is saved and a client sends `PATCH /api/preferences` with only `categorization_temperature` `0.5`
- **THEN** the context length is still Auto

#### Scenario: Old default stored
- **WHEN** an earlier version stored `categorization_context_length` `4096`
- **THEN** it reads as Auto, with `categorization_context_length_is_auto` `true`

#### Scenario: 4096 saved on purpose
- **WHEN** a client saves `categorization_context_length` `4096`
- **THEN** it reads as `4096` with `categorization_context_length_is_auto` `false`

### Requirement: AI Provider Preference
`ai_provider` SHALL be `"ollama"` or `"anthropic"`, else rejected with HTTP status 400 and the message `AI provider must be "ollama" or "anthropic" (got "<value>")`. Saving `"anthropic"` while `anthropic_available` is `false` SHALL be rejected with HTTP status 400 and the message `Set YARRTUBE_ANTHROPIC_API_KEY to use Anthropic`. Every AI feature SHALL run on the saved provider.

#### Scenario: Choosing Anthropic
- **WHEN** a key is set and a client sends `PATCH /api/preferences` with `ai_provider` `"anthropic"`
- **THEN** the daemon responds with HTTP status 200 and `ai_provider` `"anthropic"`

#### Scenario: Anthropic without a key
- **WHEN** no key is set and a client sends `PATCH /api/preferences` with `ai_provider` `"anthropic"`
- **THEN** the daemon responds with HTTP status 400 and the message `Set YARRTUBE_ANTHROPIC_API_KEY to use Anthropic`, and nothing is saved

#### Scenario: Unknown provider
- **WHEN** a client sends `PATCH /api/preferences` with `ai_provider` `"openai"`
- **THEN** the daemon responds with HTTP status 400 and the message `AI provider must be "ollama" or "anthropic" (got "openai")`

#### Scenario: Key removed after choosing Anthropic
- **WHEN** `ai_provider` `"anthropic"` was saved and the daemon restarts without a key
- **THEN** `ai_provider` still reads as `"anthropic"`, `anthropic_available` as `false`, and AI steps wait as when Anthropic is unavailable

### Requirement: Anthropic Availability
`anthropic_available` SHALL read as `true` when the daemon has an Anthropic key (see `llm-connection`) and `false` otherwise. It SHALL be read-only: a `PATCH /api/preferences` that sends it SHALL ignore it.

#### Scenario: Key set
- **WHEN** the daemon runs with `YARRTUBE_ANTHROPIC_API_KEY` set and a client requests `GET /api/preferences`
- **THEN** `anthropic_available` is `true` and no field holds the key

#### Scenario: Sent by a client
- **WHEN** no key is set and a client sends `PATCH /api/preferences` with `anthropic_available` `true`
- **THEN** the daemon responds with HTTP status 200 and `anthropic_available` `false`

### Requirement: Anthropic Generation Settings
`anthropic_summaries_model` and `anthropic_categorization_model` SHALL be non-blank names of at most 200 characters, else rejected with HTTP status 400 and `Summary model must be named (got "<value>")` or `Categorization model must be named (got "<value>")`. Saving them SHALL NOT change the Ollama settings, and the reverse.

#### Scenario: Saving Anthropic settings
- **WHEN** a client sends `PATCH /api/preferences` with `anthropic_summaries_model` `"claude-sonnet-5-5"` and `anthropic_categorization_effort` `"low"`
- **THEN** the daemon responds with HTTP status 200 and those values, and `summaries_model` and `categorization_model` are unchanged

#### Scenario: Blank model
- **WHEN** a client sends `PATCH /api/preferences` with `anthropic_categorization_model` `""`
- **THEN** the daemon responds with HTTP status 400 and the message `Categorization model must be named (got "")`

#### Scenario: Ollama settings kept while on Anthropic
- **WHEN** `summaries_model` is `gemma3:4b`, the provider is switched to Anthropic and back to Ollama
- **THEN** `summaries_model` reads as `gemma3:4b`

### Requirement: Anthropic Effort
`anthropic_summaries_effort` and `anthropic_categorization_effort` SHALL be `"low"`, `"medium"` or `"high"`, else rejected with HTTP status 400 and the message `Effort must be "low", "medium" or "high" (got "<value>")`.

#### Scenario: Saving an effort
- **WHEN** a client sends `PATCH /api/preferences` with `anthropic_summaries_effort` `"high"`
- **THEN** the daemon responds with HTTP status 200 and `anthropic_summaries_effort` `"high"`

#### Scenario: Unknown effort
- **WHEN** a client sends `PATCH /api/preferences` with `anthropic_summaries_effort` `"max"`
- **THEN** the daemon responds with HTTP status 400 and the message `Effort must be "low", "medium" or "high" (got "max")`

### Requirement: Plex Playlist Preferences
The preferences SHALL include `plex_playlist_watched`, `"keep"` or `"drop"`, defaulting to `"keep"`, and `plex_playlist_order`, `"newest_first"` or `"oldest_first"`, defaulting to `"newest_first"`. They SHALL be read and saved like every other preference, whether or not Plex is configured, and read as their defaults when never saved, including after an upgrade.

#### Scenario: Fresh install
- **WHEN** no preferences have been saved and a client requests `GET /api/preferences`
- **THEN** `plex_playlist_watched` reads `"keep"` and `plex_playlist_order` reads `"newest_first"`

#### Scenario: Saving one
- **WHEN** a client sends `PATCH /api/preferences` with only `plex_playlist_watched` `"drop"`
- **THEN** the daemon responds 200 with `plex_playlist_watched` `"drop"`, every other preference unchanged

#### Scenario: Invalid value
- **WHEN** a client sends `PATCH /api/preferences` with `plex_playlist_order` `"random"`
- **THEN** the daemon responds with HTTP 400 and saves nothing

#### Scenario: Upgrading from a version without them
- **WHEN** preferences were saved by a version without the Plex playlist preferences
- **THEN** the saved preferences keep their values and both read as their defaults

### Requirement: Playlist Preference Change Requests A Label Sync
When a save changes `plex_playlist_watched` or `plex_playlist_order` and Plex is configured, the daemon SHALL make the label sync due now. A save that changes neither SHALL NOT.

#### Scenario: Watched option changed
- **WHEN** Plex is configured, the next label sync is due in 12 minutes and a client saves `plex_playlist_watched` `"drop"`
- **THEN** a label sync runs now

#### Scenario: Same value saved
- **WHEN** `plex_playlist_order` is `"newest_first"` and a client saves it as `"newest_first"`
- **THEN** no label sync is brought forward

#### Scenario: Plex not configured
- **WHEN** Plex isn't configured and a client saves `plex_playlist_watched` `"drop"`
- **THEN** the preference is saved and nothing is scheduled
