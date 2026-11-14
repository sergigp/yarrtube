# Spec Delta

## MODIFIED Requirements

### Requirement: AI Page
The web UI SHALL show, as the Settings page's "AI" section at `/settings/ai`, a section headed "AI" with a "Provider" section (an "Enable AI" switch, the provider choice described in "Provider Choice", and, while Ollama is selected, an "Ollama URL" field and a "Test connection" button), a "Summaries" section and, below it, a "Categorization" section as "Categorization Section" describes. The "Summaries" section SHALL have a switch labelled "Enabled" (accessible name "Summaries enabled"), "Model", disabled while summaries are off, and, only while summaries are on, the advanced summary settings, all filled with the saved preferences of the selected provider. The whole Summaries section SHALL be disabled while "Enable AI" is off.

#### Scenario: Fresh install
- **WHEN** a user opens `/settings/ai` and nothing has been saved
- **THEN** "Enable AI" is off, the URL is empty, "Summaries enabled" is off and disabled, "Model" shows `qwen3:4b` disabled, and "Advanced" is not shown

#### Scenario: Summaries turned on
- **WHEN** a user turns "Enable AI" on and then "Summaries enabled" on, with Ollama selected
- **THEN** "Model" shows `qwen3:4b` and "Advanced" is closed; opening it shows `12288`, `0.3`, thinking on and the shipped prompt, all editable

#### Scenario: AI turned off with summaries on
- **WHEN** summaries are on and "Enable AI" is off
- **THEN** the summary settings show but are disabled

#### Scenario: Reached from the old route
- **WHEN** a user opens `/ai`
- **THEN** the URL becomes `/settings/ai` and this section shows
