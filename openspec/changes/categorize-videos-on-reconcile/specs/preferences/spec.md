# Spec Delta

## MODIFIED Requirements

### Requirement: Preferences And Their Defaults
The daemon SHALL keep one set of preferences: `categories_enabled`, `home_show_continue_watching`, `home_show_categories`, `home_show_quick_watches` and `home_show_latest`, each a boolean that defaults to `true`; `theme`, either `"light"` or `"dark"`, that defaults to `"light"`; `ai_enabled`, `summaries_enabled` and `categorization_enabled`, booleans that default to `false`; `ollama_url`, an http(s) URL that defaults to unset (`null`); `summaries_model`, defaulting to `"qwen3:4b"`; `summaries_context_length`, defaulting to `12288`; `summaries_temperature`, defaulting to `0.3`; `summaries_thinking`, defaulting to `true`; and `summaries_prompt`, defaulting to the summary prompt shipped with the running version (`prompts/summaries/v3.md`), read alongside `summaries_prompt_is_default`; `categorization_model`, defaulting to `"qwen3:4b"`; and `categorization_prompt`, defaulting to the categorization prompt shipped with the running version (`prompts/categorization/v2.md`), read alongside `categorization_prompt_is_default`. A preference never saved SHALL read as its default, including after an upgrade that adds a new preference. A stored `theme` that is neither `"light"` nor `"dark"` SHALL read as `"light"`.

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
