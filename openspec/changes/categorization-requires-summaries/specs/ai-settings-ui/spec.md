## MODIFIED Requirements

### Requirement: Categorization Section
The "Categorization" card SHALL have a "Categorization" switch beside its heading, with the description "Suggest categories for new videos from channels and playlists that have categorization on." followed by a "Uses your categories" link to `/settings/general`. The switch SHALL be disabled while "Summaries" is off on the page, saved or not, keeping its value, and the description SHALL then be followed by "Needs summaries.". Categorization counts as on below only while both switches are on. It SHALL also have "Model", disabled while categorization is off. Only while categorization is on, it SHALL show:
- while Ollama is selected, an "Advanced" disclosure, closed each time the page opens, holding "Context length", "Temperature" and the "Thinking" switch;
- while Anthropic is selected, "Effort" directly in the card;
- for both providers, the "Prompt" disclosure.

All of it SHALL be filled with the saved preferences. The whole card SHALL be disabled while "AI features" is off, and SHALL NOT show while `categories_enabled` is off. Changes SHALL be saved with the rest of the page from the save bar, sending only the changed fields.

#### Scenario: Fresh install
- **WHEN** a user opens `/ai` and nothing has been saved
- **THEN** the "Categorization" switch is off and disabled, "Model" shows `qwen3:4b` disabled, and neither "Advanced" nor "Prompt" is shown in the card

#### Scenario: Categorization turned on
- **WHEN** a user turns "AI features" on, then "Summaries" and "Categorization" on, with Ollama selected and no category with an AI prompt
- **THEN** "Model" shows `qwen3:4b` and "Advanced" is closed; opening it shows "Auto context length" on with `4096`, `0` and thinking on, and opening "Prompt" shows the shipped prompt, all editable

#### Scenario: Summaries off
- **WHEN** AI is on, summaries are off and a user opens `/ai`
- **THEN** the "Categorization" switch is disabled, the card reads "Needs summaries.", and "Model" is disabled

#### Scenario: Turning summaries off on the page
- **WHEN** AI, summaries and categorization are saved on and a user turns "Summaries" off without saving
- **THEN** the "Categorization" switch stays on but is disabled, the card reads "Needs summaries.", "Model" is disabled and neither "Advanced" nor "Prompt" is shown

#### Scenario: Saving with summaries off
- **WHEN** AI, summaries and categorization are saved on, and a user turns "Summaries" off and activates "Save changes"
- **THEN** a `PATCH /api/preferences` with only `summaries_enabled` `false` is sent

#### Scenario: Saving categorization
- **WHEN** summaries are on and a user turns "Categorization" on and activates "Save changes"
- **THEN** a `PATCH /api/preferences` with only `categorization_enabled` `true` is sent and "Saved" is shown

#### Scenario: Saving the advanced settings
- **WHEN** a user opens the categorization "Advanced", turns "Auto context length" off, changes "Context length" to `8192` and activates "Save changes"
- **THEN** a `PATCH /api/preferences` with only `categorization_context_length` `8192` is sent, and the summary context length is unchanged

#### Scenario: Not a number
- **WHEN** a user empties the categorization "Temperature" and activates "Save changes"
- **THEN** nothing is sent and the bar says `Context length and temperature must be numbers`

#### Scenario: Categories off
- **WHEN** `categories_enabled` is off and a user opens `/ai`
- **THEN** the "Categorization" card is not shown

#### Scenario: Anthropic categorization settings
- **WHEN** Anthropic is selected and summaries and categorization are on
- **THEN** the card shows "Effort" and the "Prompt" disclosure, and no "Advanced", context length, temperature or thinking
