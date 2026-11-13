# ai-settings-ui Specification

## Purpose
The web UI page where the user turns AI features on, points Yarrtube at an Ollama server, and configures how video summaries are generated, including the prompt.

## Requirements

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
- **THEN** it shows "Effort" set to `low` and the prompt, and no context length, temperature or thinking

### Requirement: Save AI Settings
The page SHALL have a "Save" button, enabled only when a field differs from the saved preferences, that saves only the changed fields. On success it SHALL show "Saved"; on failure it SHALL show the daemon's error message and keep the edited values.

#### Scenario: Saving the URL and turning AI on
- **WHEN** a user types `http://mac.local:11434`, turns "Enable AI" on and activates "Save"
- **THEN** a `PATCH /api/preferences` with only `ai_enabled` and `ollama_url` is sent and "Saved" is shown

#### Scenario: Rejected save
- **WHEN** a user turns "Enable AI" on with an empty URL and activates "Save"
- **THEN** the page shows `Ollama URL must be set to enable AI` and "Enable AI" stays on in the form

#### Scenario: Prompt untouched
- **WHEN** a user changes only the temperature and activates "Save"
- **THEN** the request does not include `summaries_prompt`

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

### Requirement: Thinking Switch
In each of the Summaries and Categorization sections, the "Thinking" switch SHALL show unless the section's selected model is known not to support thinking; then the page SHALL show `<model> doesn't support thinking` instead, keeping the saved value. When models are unknown or the model isn't listed, the switch SHALL show. Its help SHALL say thinking makes the model reason before answering (slower, more accurate) and that some models, such as `qwen3:4b`, think even when it is off.

#### Scenario: Model that can think
- **WHEN** the selected model is `qwen3:4b` and the server reports it can think
- **THEN** the "Thinking" switch is shown

#### Scenario: Model that can't think
- **WHEN** a user selects `gemma3:4b` and the server reports it can't think
- **THEN** the switch is hidden, `gemma3:4b doesn't support thinking` is shown, and saving does not send `summaries_thinking`

#### Scenario: Sections decide independently
- **WHEN** the summaries model can think and a user selects `gemma3:4b`, which can't, for categorization
- **THEN** the summaries "Thinking" switch stays, the categorization one is replaced by `gemma3:4b doesn't support thinking`, and saving does not send `categorization_thinking`

#### Scenario: Models unknown
- **WHEN** checking the saved URL failed and "Model" is a text field
- **THEN** the "Thinking" switch is shown

#### Scenario: Reading the help
- **WHEN** a user opens the "Thinking" help
- **THEN** it says thinking is slower but more accurate and that `qwen3:4b` thinks even when off

### Requirement: Prompt Editor
"Prompt" SHALL be a multi-line text field with the whole prompt document. When the prompt is the shipped one, the page SHALL say `Using the default prompt`. A "Reset to default" button, shown only for a saved prompt, SHALL save `summaries_prompt` as `null` and show the shipped prompt.

#### Scenario: Default prompt
- **WHEN** a user opens `/ai` and no prompt has been saved
- **THEN** the prompt field holds the shipped prompt, `Using the default prompt` is shown and "Reset to default" is not

#### Scenario: Resetting a saved prompt
- **WHEN** a prompt is saved and a user activates "Reset to default"
- **THEN** a `PATCH /api/preferences` with `summaries_prompt` `null` is sent, the field holds the shipped prompt and `Using the default prompt` is shown

#### Scenario: Invalid prompt
- **WHEN** a user deletes the `## map user` section and activates "Save"
- **THEN** the page shows `Prompt is missing the "map user" section` and the field keeps the edited text

### Requirement: Prompt Guide Access
A "?" button with the accessible name "About Prompt", next to the "Prompt" label, SHALL open the prompt guide in a scrollable popover; the guide SHALL NOT show until it is opened.

#### Scenario: Guide hidden until asked for
- **WHEN** a user opens `/ai`
- **THEN** the guide is not shown until they activate "About Prompt"

### Requirement: Prompt Guide
The prompt guide SHALL explain that a video that fits the context length uses `system` + `user`, a longer one is cut into parts summarized with `map system` + `map user` and then combined with `reduce system` + `reduce user`, and that `chapters`, `map chapters` and `reduce part` are snippets inserted into those messages. For each section it SHALL list the variables it can use, each with what it is replaced by.

#### Scenario: Variables for a section
- **WHEN** a user opens the guide and reads `reduce part`
- **THEN** it lists `{part}`, `{span}` and `{summary}`, each with a description

#### Scenario: Every variable described
- **WHEN** a user reads the guide
- **THEN** every variable allowed by the daemon is listed under each section that allows it, with a description

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

### Requirement: Categorization Prompt Editor
The categorization "Prompt" SHALL be a multi-line text field with the whole prompt, sent to the model before each category's own prompt. When the prompt is the shipped one, the section SHALL say `Using the default prompt`. A "Reset to default" button, shown only for a saved prompt, SHALL save `categorization_prompt` as `null` and show the shipped prompt. There is no prompt guide, since the prompt has no sections or variables.

#### Scenario: Default prompt
- **WHEN** a user opens the categorization "Advanced" and no prompt has been saved
- **THEN** the prompt field holds the shipped prompt, `Using the default prompt` is shown and "Reset to default" is not

#### Scenario: Saving an edited prompt
- **WHEN** a user edits the prompt and activates "Save"
- **THEN** a `PATCH /api/preferences` with only `categorization_prompt` is sent

#### Scenario: Resetting a saved prompt
- **WHEN** a prompt is saved and a user activates "Reset to default" in the section
- **THEN** a `PATCH /api/preferences` with `categorization_prompt` `null` is sent, the field holds the shipped prompt and `Using the default prompt` is shown

#### Scenario: Blank prompt
- **WHEN** a user empties the prompt and activates "Save"
- **THEN** the page shows `Categorization prompt must not be empty` and the field stays empty

### Requirement: Auto Categorization Context Length Choice
The categorization "Context length" SHALL have a switch labelled "Auto" (accessible name "Auto context length"), on when `categorization_context_length_is_auto` is `true`. While it is on, "Context length" SHALL show the Auto value, read-only. Turning it off SHALL make the field editable, starting at that value. Turning it on and saving SHALL send `categorization_context_length` `null`. Below the field the section SHALL show a hint on how Auto scales.

#### Scenario: Auto shown
- **WHEN** a user opens the categorization "Advanced", the context length is Auto and 9 categories have an AI prompt
- **THEN** "Auto context length" is on, "Context length" shows `16384` read-only, and below it the page says `Auto sizes it from your categories with an AI prompt, about 1,800 tokens each. Too small and videos are asked about one category at a time, which is slower.`

#### Scenario: Overriding
- **WHEN** a user turns "Auto context length" off
- **THEN** "Context length" is editable and holds `16384`

#### Scenario: Back to Auto
- **WHEN** `8192` is saved, a user turns "Auto context length" on and activates "Save"
- **THEN** a `PATCH /api/preferences` with only `categorization_context_length` `null` is sent, and "Context length" shows the Auto value, read-only

#### Scenario: Turned off and on without saving
- **WHEN** the context length is Auto, a user turns "Auto context length" off then on again and activates "Save"
- **THEN** no `categorization_context_length` is sent

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
