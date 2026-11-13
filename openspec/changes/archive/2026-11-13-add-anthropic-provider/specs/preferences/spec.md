## MODIFIED Requirements

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

## ADDED Requirements

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
