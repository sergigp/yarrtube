## MODIFIED Requirements

### Requirement: Categorization Section
The "Categorization" section SHALL have a switch labelled "Enabled" (accessible name "Categorization enabled"), "Model", disabled while categorization is off, and, only while categorization is on, an "Advanced" disclosure holding "Context length", "Temperature", the "Thinking" switch and "Prompt", closed each time the page opens, all filled with the saved preferences. The whole section SHALL be disabled while "Enable AI" is off, and SHALL NOT show while `categories_enabled` is off. Changes SHALL be saved by the page's "Save" button with the rest of the page, sending only the changed fields.

#### Scenario: Fresh install
- **WHEN** a user opens `/ai` and nothing has been saved
- **THEN** "Categorization enabled" is off and disabled, "Model" shows `qwen3:4b` disabled, and "Advanced" is not shown in the section

#### Scenario: Categorization turned on
- **WHEN** a user turns "Enable AI" on and then "Categorization enabled" on
- **THEN** "Model" shows `qwen3:4b` and "Advanced" is closed; opening it shows `4096`, `0`, thinking on and the shipped prompt, all editable

#### Scenario: Saving categorization
- **WHEN** a user turns "Categorization enabled" on and activates "Save"
- **THEN** a `PATCH /api/preferences` with only `categorization_enabled` `true` is sent and "Saved" is shown

#### Scenario: Saving the advanced settings
- **WHEN** a user opens the categorization "Advanced", changes "Context length" to `8192` and activates "Save"
- **THEN** a `PATCH /api/preferences` with only `categorization_context_length` `8192` is sent, and the summary context length is unchanged

#### Scenario: Not a number
- **WHEN** a user empties the categorization "Temperature" and activates "Save"
- **THEN** nothing is sent and the page says `Context length and temperature must be numbers`

#### Scenario: Categories off
- **WHEN** `categories_enabled` is off and a user opens `/ai`
- **THEN** the "Categorization" section is not shown

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
