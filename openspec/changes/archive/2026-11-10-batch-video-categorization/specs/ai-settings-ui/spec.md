# Spec Delta

## MODIFIED Requirements

### Requirement: Categorization Section
The "Categorization" section SHALL have a switch labelled "Enabled" (accessible name "Categorization enabled"), "Model", disabled while categorization is off, and, only while categorization is on, an "Advanced" disclosure holding "Context length", "Temperature", the "Thinking" switch and "Prompt", closed each time the page opens, all filled with the saved preferences. The whole section SHALL be disabled while "Enable AI" is off, and SHALL NOT show while `categories_enabled` is off. Changes SHALL be saved by the page's "Save" button with the rest of the page, sending only the changed fields.

#### Scenario: Fresh install
- **WHEN** a user opens `/ai` and nothing has been saved
- **THEN** "Categorization enabled" is off and disabled, "Model" shows `qwen3:4b` disabled, and "Advanced" is not shown in the section

#### Scenario: Categorization turned on
- **WHEN** a user turns "Enable AI" on and then "Categorization enabled" on, with no category with an AI prompt
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

## ADDED Requirements

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
