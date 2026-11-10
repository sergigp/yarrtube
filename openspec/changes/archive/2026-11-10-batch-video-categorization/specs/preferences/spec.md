# Spec Delta

## MODIFIED Requirements

### Requirement: Preferences And Their Defaults
The daemon SHALL keep one set of preferences: `categories_enabled`, `home_show_continue_watching`, `home_show_categories`, `home_show_quick_watches` and `home_show_latest`, each a boolean that defaults to `true`; `theme`, either `"light"` or `"dark"`, that defaults to `"light"`; `ai_enabled`, `summaries_enabled` and `categorization_enabled`, booleans that default to `false`; `ollama_url`, an http(s) URL that defaults to unset (`null`); `summaries_model`, defaulting to `"qwen3:4b"`; `summaries_context_length`, defaulting to `12288`; `summaries_temperature`, defaulting to `0.3`; `summaries_thinking`, defaulting to `true`; and `summaries_prompt`, defaulting to the summary prompt shipped with the running version (`prompts/summaries/v3.md`), read alongside `summaries_prompt_is_default`; `categorization_model`, defaulting to `"qwen3:4b"`; `categorization_context_length`, defaulting to Auto (see "Auto Categorization Context Length"), read alongside `categorization_context_length_is_auto`; `categorization_temperature`, defaulting to `0`; `categorization_thinking`, defaulting to `true`; and `categorization_prompt`, defaulting to the categorization prompt shipped with the running version (`prompts/categorization/v2-batch.md`), read alongside `categorization_prompt_is_default`. A preference never saved SHALL read as its default, including after an upgrade that adds a new preference. A stored `theme` that is neither `"light"` nor `"dark"` SHALL read as `"light"`.

#### Scenario: Fresh install
- **WHEN** no preferences have been saved and no category has an AI prompt
- **THEN** every display boolean preference reads as `true`, `theme` reads as `"light"`, `ai_enabled`, `summaries_enabled` and `categorization_enabled` read as `false`, `ollama_url` reads as `null`, the summary settings read as `qwen3:4b`, `12288`, `0.3`, thinking on and the shipped prompt, and the categorization settings read as `qwen3:4b`, `4096` with `categorization_context_length_is_auto` `true`, `0`, thinking on and the shipped prompt

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

## ADDED Requirements

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
