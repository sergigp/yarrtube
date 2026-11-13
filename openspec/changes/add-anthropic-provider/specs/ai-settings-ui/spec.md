## MODIFIED Requirements

### Requirement: AI Page
The web UI SHALL show, at `/ai`, a page headed "AI" with a "Provider" section (an "Enable AI" switch, the provider choice described in "Provider Choice", and, while Ollama is selected, an "Ollama URL" field and a "Test connection" button), a "Summaries" section and, below it, a "Categorization" section as "Categorization Section" describes. The "Summaries" section SHALL have a switch labelled "Enabled" (accessible name "Summaries enabled"), "Model", disabled while summaries are off, and, only while summaries are on, the advanced summary settings, all filled with the saved preferences of the selected provider. The whole Summaries section SHALL be disabled while "Enable AI" is off.

#### Scenario: Fresh install
- **WHEN** a user opens `/ai` and nothing has been saved
- **THEN** "Enable AI" is off, the URL is empty, "Summaries enabled" is off and disabled, "Model" shows `qwen3:4b` disabled, and "Advanced" is not shown

#### Scenario: Summaries turned on
- **WHEN** a user turns "Enable AI" on and then "Summaries enabled" on, with Ollama selected
- **THEN** "Model" shows `qwen3:4b` and "Advanced" is closed; opening it shows `12288`, `0.3`, thinking on and the shipped prompt, all editable

#### Scenario: AI turned off with summaries on
- **WHEN** summaries are on and "Enable AI" is off
- **THEN** the summary settings show but are disabled

### Requirement: Advanced Summary Settings
While Ollama is selected, "Context length", "Temperature", the "Thinking" switch and "Prompt" SHALL sit under an "Advanced" disclosure; while Anthropic is selected, "Effort" and "Prompt" SHALL. The disclosure SHALL be closed each time the page opens, whatever their values.

#### Scenario: Closed by default
- **WHEN** a user opens `/ai` with summaries on and a saved custom prompt
- **THEN** "Advanced" is closed and the prompt is not shown until it is opened

#### Scenario: Anthropic advanced settings
- **WHEN** Anthropic is selected, summaries are on and a user opens "Advanced"
- **THEN** it shows "Effort" set to `medium` and the prompt, and no context length, temperature or thinking

### Requirement: Test Ollama Connection
While Ollama is selected, activating "Test connection" SHALL check the URL currently typed, saved or not, and show `Connected to Ollama <version>, <n> models` on success or the daemon's error message on failure. The button SHALL be disabled while the URL field is empty or a check is running.

#### Scenario: Reachable server
- **WHEN** a user types `http://mac.local:11434` and activates "Test connection", and Ollama 0.40.13 there has 2 models
- **THEN** the page shows `Connected to Ollama 0.40.13, 2 models`

#### Scenario: Unreachable server
- **WHEN** a user activates "Test connection" and the daemon answers with HTTP status 502
- **THEN** the page shows the daemon's error message

### Requirement: Model Picker
While Ollama is selected, in each of the Summaries and Categorization sections, when the models of the saved or last tested URL are known, "Model" SHALL be a list of their names; a selected model the server does not have SHALL still be listed, marked `(not installed)`. When no models are known, "Model" SHALL be a text field. On load, with a saved URL, the page SHALL check it once to fetch its models.

#### Scenario: Models listed
- **WHEN** a user opens `/ai` with a saved URL whose server has `gemma3:4b` and `qwen3:4b`
- **THEN** "Model" lists `gemma3:4b` and `qwen3:4b` with `qwen3:4b` selected

#### Scenario: Saved model not installed
- **WHEN** the saved model is `qwen3:4b` and the server only has `gemma3:4b`
- **THEN** "Model" lists `gemma3:4b` and `qwen3:4b (not installed)`, with the latter selected

#### Scenario: Categorization model listed
- **WHEN** a user turns categorization on with a saved URL whose server has `gemma3:4b` and `qwen3:4b`
- **THEN** the Categorization "Model" lists `gemma3:4b` and `qwen3:4b` with `qwen3:4b` selected, independently of the Summaries "Model"

#### Scenario: Server down
- **WHEN** a user opens `/ai` and checking the saved URL fails
- **THEN** "Model" is a text field holding the saved model

### Requirement: Categorization Section
The "Categorization" section SHALL have a switch labelled "Enabled" (accessible name "Categorization enabled"), "Model", disabled while categorization is off, and, only while categorization is on, an "Advanced" disclosure, closed each time the page opens, holding, while Ollama is selected, "Context length", "Temperature", the "Thinking" switch and "Prompt", and while Anthropic is selected, "Effort" and "Prompt", all filled with the saved preferences. The whole section SHALL be disabled while "Enable AI" is off, and SHALL NOT show while `categories_enabled` is off. Changes SHALL be saved by the page's "Save" button with the rest of the page, sending only the changed fields.

#### Scenario: Fresh install
- **WHEN** a user opens `/ai` and nothing has been saved
- **THEN** "Categorization enabled" is off and disabled, "Model" shows `qwen3:4b` disabled, and "Advanced" is not shown in the section

#### Scenario: Categorization turned on
- **WHEN** a user turns "Enable AI" on and then "Categorization enabled" on, with Ollama selected and no category with an AI prompt
- **THEN** "Model" shows `qwen3:4b` and "Advanced" is closed; opening it shows "Auto context length" on with `4096`, `0`, thinking on and the shipped prompt, all editable

#### Scenario: Saving categorization
- **WHEN** a user turns "Categorization enabled" on and activates "Save"
- **THEN** a `PATCH /api/preferences` with only `categorization_enabled` `true` is sent and "Saved" is shown

#### Scenario: Saving the advanced settings
- **WHEN** a user opens the categorization "Advanced", turns "Auto context length" off, changes "Context length" to `8192` and activates "Save"
- **THEN** a `PATCH /api/preferences` with only `categorization_context_length` `8192` is sent, and the summary context length is unchanged

#### Scenario: Not a number
- **WHEN** a user empties the categorization "Temperature" and activates "Save"
- **THEN** nothing is sent and the page says `Context length and temperature must be numbers`

#### Scenario: Categories off
- **WHEN** `categories_enabled` is off and a user opens `/ai`
- **THEN** the "Categorization" section is not shown

#### Scenario: Anthropic categorization settings
- **WHEN** Anthropic is selected, categorization is on and a user opens its "Advanced"
- **THEN** it shows "Effort" and the prompt, and no context length, temperature or thinking

## ADDED Requirements

### Requirement: Provider Choice
While `anthropic_available` is `true`, the "Provider" section SHALL offer a choice between "Ollama" and "Anthropic", set to the saved `ai_provider`; changing it SHALL be saved by "Save" as `ai_provider`. While `anthropic_available` is `false`, no choice SHALL show, unless `"anthropic"` is saved: then the section SHALL say `Anthropic is selected but YARRTUBE_ANTHROPIC_API_KEY isn't set` and offer only "Ollama".

#### Scenario: No key
- **WHEN** `anthropic_available` is `false` and the saved provider is Ollama
- **THEN** no provider choice is shown and the section looks as before

#### Scenario: Switching to Anthropic
- **WHEN** `anthropic_available` is `true`, a user selects "Anthropic" and activates "Save"
- **THEN** a `PATCH /api/preferences` with only `ai_provider` `"anthropic"` is sent, the Ollama URL field is hidden and the sections show the Anthropic settings

#### Scenario: Key removed
- **WHEN** the saved provider is `"anthropic"` and `anthropic_available` is `false`
- **THEN** the section says `Anthropic is selected but YARRTUBE_ANTHROPIC_API_KEY isn't set` and offers "Ollama"

### Requirement: Test Anthropic Connection
While Anthropic is selected, activating "Test connection" SHALL check Anthropic and show `Connected to Anthropic, <n> models` on success or the daemon's error message on failure. The button SHALL be disabled while a check is running.

#### Scenario: Key works
- **WHEN** a user selects "Anthropic" and activates "Test connection", and the key can use 4 models
- **THEN** the page shows `Connected to Anthropic, 4 models`

#### Scenario: Key rejected
- **WHEN** a user activates "Test connection" and the daemon answers with HTTP status 502
- **THEN** the page shows the daemon's error message

### Requirement: Anthropic Model Picker
While Anthropic is selected, in each of the Summaries and Categorization sections, when Anthropic's models are known, "Model" SHALL list their ids in the order the daemon gives; a selected id not in the list SHALL still be listed, marked `(not available)`. When the models are unknown, "Model" SHALL be a text field. On load with Anthropic saved, the page SHALL check Anthropic once to fetch its models.

#### Scenario: Models listed
- **WHEN** a user opens `/ai` with Anthropic saved and the key can use `claude-sonnet-5-5` and `claude-haiku-5-5`
- **THEN** "Model" lists `claude-sonnet-5-5` then `claude-haiku-5-5`, with `claude-haiku-5-5` selected

#### Scenario: Retired model
- **WHEN** the saved Anthropic summaries model is `claude-haiku-4-5` and Anthropic no longer lists it
- **THEN** "Model" lists Anthropic's models and `claude-haiku-4-5 (not available)`, with the latter selected

#### Scenario: Check failed
- **WHEN** checking Anthropic fails on load
- **THEN** "Model" is a text field holding the saved Anthropic model

### Requirement: Effort Field
While Anthropic is selected, each section's "Advanced" SHALL hold "Effort", a choice of `low`, `medium` and `high`, filled with the section's saved effort and saved by "Save" as `anthropic_summaries_effort` or `anthropic_categorization_effort`. Its help SHALL say higher effort makes the model reason more, which is slower and costs more tokens.

#### Scenario: Changing effort
- **WHEN** Anthropic is selected and a user sets the summaries "Effort" to `low` and activates "Save"
- **THEN** a `PATCH /api/preferences` with only `anthropic_summaries_effort` `"low"` is sent
