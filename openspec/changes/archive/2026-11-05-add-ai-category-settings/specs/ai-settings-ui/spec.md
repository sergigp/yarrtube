## MODIFIED Requirements

### Requirement: AI Page
The web UI SHALL show, at `/ai`, a page headed "AI" with an "Ollama" section (an "Enable AI" switch, an "Ollama URL" field and a "Test connection" button), a "Summaries" section and, below it, a "Categorization" section as "Categorization Section" describes. The "Summaries" section SHALL have a switch labelled "Enabled" (accessible name "Summaries enabled"), "Model", disabled while summaries are off, and, only while summaries are on, the advanced summary settings, all filled with the saved preferences. The whole Summaries section SHALL be disabled while "Enable AI" is off.

#### Scenario: Fresh install
- **WHEN** a user opens `/ai` and nothing has been saved
- **THEN** "Enable AI" is off, the URL is empty, "Summaries enabled" is off and disabled, "Model" shows `qwen3:4b` disabled, and "Advanced" is not shown

#### Scenario: Summaries turned on
- **WHEN** a user turns "Enable AI" on and then "Summaries enabled" on
- **THEN** "Model" shows `qwen3:4b` and "Advanced" is closed; opening it shows `12288`, `0.3`, thinking on and the shipped prompt, all editable

#### Scenario: AI turned off with summaries on
- **WHEN** summaries are on and "Enable AI" is off
- **THEN** the summary settings show but are disabled

### Requirement: Model Picker
In each of the Summaries and Categorization sections, when the models of the saved or last tested URL are known, "Model" SHALL be a list of their names; a selected model the server does not have SHALL still be listed, marked `(not installed)`. When no models are known, "Model" SHALL be a text field. On load, with a saved URL, the page SHALL check it once to fetch its models.

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

## ADDED Requirements

### Requirement: Categorization Section
The "Categorization" section SHALL have a switch labelled "Enabled" (accessible name "Categorization enabled"), "Model", disabled while categorization is off, and, only while categorization is on, an "Advanced" disclosure holding "Prompt", closed each time the page opens, all filled with the saved preferences. The whole section SHALL be disabled while "Enable AI" is off, and SHALL NOT show while `categories_enabled` is off. Changes SHALL be saved by the page's "Save" button with the rest of the page, sending only the changed fields.

#### Scenario: Fresh install
- **WHEN** a user opens `/ai` and nothing has been saved
- **THEN** "Categorization enabled" is off and disabled, "Model" shows `qwen3:4b` disabled, and "Advanced" is not shown in the section

#### Scenario: Categorization turned on
- **WHEN** a user turns "Enable AI" on and then "Categorization enabled" on
- **THEN** "Model" shows `qwen3:4b` and "Advanced" is closed; opening it shows the shipped prompt, editable

#### Scenario: Saving categorization
- **WHEN** a user turns "Categorization enabled" on and activates "Save"
- **THEN** a `PATCH /api/preferences` with only `categorization_enabled` `true` is sent and "Saved" is shown

#### Scenario: Categories off
- **WHEN** `categories_enabled` is off and a user opens `/ai`
- **THEN** the "Categorization" section is not shown

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
