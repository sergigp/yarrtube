## MODIFIED Requirements

### Requirement: AI Page
The web UI SHALL show, as the Settings page's "AI" section at `/settings/ai`, a page titled "AI". Beside the title SHALL be an "AI features" switch. Under the title SHALL be "Summaries and automatic categorization, run on a local Ollama server or on Anthropic." and an "AI guide" link to `doc/AI.md` on GitHub. Below them SHALL come these cards, in order:
- "Provider": the provider choice described in "Provider Choice", plus, while Ollama is selected, an "Ollama URL" field and a "Test connection" button.
- "Summaries".
- "Categorization", as "Categorization Section" describes.

The "Summaries" card SHALL have a "Summaries" switch beside its heading, with the description "Summarize videos from channels and playlists that have summaries on. The summary shows on the video's page." It SHALL also have "Model", disabled while summaries are off, and, only while summaries are on, the summary settings of the selected provider, all filled with that provider's saved preferences. The whole Summaries card SHALL be disabled while "AI features" is off.

#### Scenario: Fresh install
- **WHEN** a user opens `/settings/ai` and nothing has been saved
- **THEN** "AI features" is off, the URL is empty, the "Summaries" switch is off and disabled, "Model" shows `qwen3:4b` disabled, and neither "Advanced" nor "Prompt" is shown

#### Scenario: Summaries turned on
- **WHEN** a user turns "AI features" on and then "Summaries" on, with Ollama selected
- **THEN** "Model" shows `qwen3:4b`, and "Advanced" and "Prompt" are closed; opening "Advanced" shows `12288`, `0.3` and thinking on, and opening "Prompt" shows the shipped prompt, all editable

#### Scenario: AI turned off with summaries on
- **WHEN** summaries are on and "AI features" is off
- **THEN** the summary settings show but are disabled

#### Scenario: Reached from the old route
- **WHEN** a user opens `/ai`
- **THEN** the URL becomes `/settings/ai` and this section shows

### Requirement: Advanced Summary Settings
While Ollama is selected, "Context length", "Temperature" and the "Thinking" switch SHALL sit under an "Advanced" disclosure. While Anthropic is selected, "Effort" SHALL show directly in the card and no "Advanced" disclosure SHALL show. "Prompt" SHALL sit under its own disclosure for both providers, as "Prompt Editor" describes. Each disclosure SHALL be closed each time the page opens, whatever the values inside it.

#### Scenario: Closed by default
- **WHEN** a user opens `/ai` with summaries on and a saved custom prompt
- **THEN** "Advanced" and "Prompt" are closed and the prompt is not shown until "Prompt" is opened

#### Scenario: Anthropic advanced settings
- **WHEN** Anthropic is selected and summaries are on
- **THEN** the card shows "Effort" set to `low` without opening anything, shows no "Advanced", and shows no context length, temperature or thinking

### Requirement: Save AI Settings
While a field differs from the saved preferences, a bar SHALL stay visible at the bottom of the viewport. The bar SHALL say "<n> unsaved change" or "<n> unsaved changes" and name the changed settings, e.g. "Summaries effort, Categorization prompt". It SHALL have "Discard" and "Save changes" buttons. "Save changes" SHALL save only the changed fields. On success the bar SHALL hide and "Saved" SHALL show next to the "AI" title for about 2 seconds. On failure the bar SHALL show the daemon's error message and keep the edited values. "Discard" SHALL put every field back to its saved value and hide the bar, without a request. No bar SHALL show while nothing differs.

#### Scenario: Saving the URL and turning AI on
- **WHEN** a user types `http://mac.local:11434`, turns "AI features" on and activates "Save changes"
- **THEN** a `PATCH /api/preferences` with only `ai_enabled` and `ollama_url` is sent, the bar hides and "Saved" shows next to "AI"

#### Scenario: Listing changes
- **WHEN** a user changes the summaries temperature and the categorization prompt
- **THEN** the bar says "2 unsaved changes" and names "Summaries temperature, Categorization prompt"

#### Scenario: Discarding
- **WHEN** a user changes the summaries temperature and activates "Discard"
- **THEN** the temperature shows its saved value, the bar hides and no request is sent

#### Scenario: Rejected save
- **WHEN** a user turns "AI features" on with an empty URL and activates "Save changes"
- **THEN** the bar shows `Ollama URL must be set to enable AI` and "AI features" stays on in the form

#### Scenario: Prompt untouched
- **WHEN** a user changes only the temperature and activates "Save changes"
- **THEN** the request does not include `summaries_prompt`

### Requirement: Prompt Editor
"Prompt" SHALL be a disclosure whose header reads "Prompt · default" when the field holds the shipped prompt unchanged, and "Prompt · edited" otherwise. Opened, it SHALL show a multi-line text field with the whole prompt document. A "Reset to default" button, shown only for a saved prompt, SHALL save `summaries_prompt` as `null` and show the shipped prompt.

#### Scenario: Default prompt
- **WHEN** a user opens `/ai` and no prompt has been saved
- **THEN** the disclosure reads "Prompt · default"; opened, the field holds the shipped prompt and "Reset to default" is not shown

#### Scenario: Editing marks it edited
- **WHEN** a user types into the default prompt
- **THEN** the disclosure reads "Prompt · edited"

#### Scenario: Resetting a saved prompt
- **WHEN** a prompt is saved and a user activates "Reset to default"
- **THEN** a `PATCH /api/preferences` with `summaries_prompt` `null` is sent, the field holds the shipped prompt and the disclosure reads "Prompt · default"

#### Scenario: Invalid prompt
- **WHEN** a user deletes the `## map user` section and activates "Save changes"
- **THEN** the bar shows `Prompt is missing the "map user" section` and the field keeps the edited text

### Requirement: Categorization Section
The "Categorization" card SHALL have a "Categorization" switch beside its heading, with the description "Suggest categories for new videos from channels and playlists that have categorization on." followed by a "Uses your categories" link to `/settings/general`. It SHALL also have "Model", disabled while categorization is off. Only while categorization is on, it SHALL show:
- while Ollama is selected, an "Advanced" disclosure, closed each time the page opens, holding "Context length", "Temperature" and the "Thinking" switch;
- while Anthropic is selected, "Effort" directly in the card;
- for both providers, the "Prompt" disclosure.

All of it SHALL be filled with the saved preferences. The whole card SHALL be disabled while "AI features" is off, and SHALL NOT show while `categories_enabled` is off. Changes SHALL be saved with the rest of the page from the save bar, sending only the changed fields.

#### Scenario: Fresh install
- **WHEN** a user opens `/ai` and nothing has been saved
- **THEN** the "Categorization" switch is off and disabled, "Model" shows `qwen3:4b` disabled, and neither "Advanced" nor "Prompt" is shown in the card

#### Scenario: Categorization turned on
- **WHEN** a user turns "AI features" on and then "Categorization" on, with Ollama selected and no category with an AI prompt
- **THEN** "Model" shows `qwen3:4b` and "Advanced" is closed; opening it shows "Auto context length" on with `4096`, `0` and thinking on, and opening "Prompt" shows the shipped prompt, all editable

#### Scenario: Saving categorization
- **WHEN** a user turns "Categorization" on and activates "Save changes"
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
- **WHEN** Anthropic is selected and categorization is on
- **THEN** the card shows "Effort" and the "Prompt" disclosure, and no "Advanced", context length, temperature or thinking

### Requirement: Categorization Prompt Editor
The categorization "Prompt" SHALL be a disclosure whose header reads "Prompt · default" or "Prompt · edited", as for summaries. Opened, it SHALL show a multi-line text field with the whole prompt, sent to the model before each category's own prompt. A "Reset to default" button, shown only for a saved prompt, SHALL save `categorization_prompt` as `null` and show the shipped prompt. There is no prompt guide, since the prompt has no sections or variables.

#### Scenario: Default prompt
- **WHEN** a user opens the categorization "Prompt" and no prompt has been saved
- **THEN** it reads "Prompt · default", the field holds the shipped prompt and "Reset to default" is not shown

#### Scenario: Saving an edited prompt
- **WHEN** a user edits the prompt and activates "Save changes"
- **THEN** a `PATCH /api/preferences` with only `categorization_prompt` is sent

#### Scenario: Resetting a saved prompt
- **WHEN** a prompt is saved and a user activates "Reset to default" in the card
- **THEN** a `PATCH /api/preferences` with `categorization_prompt` `null` is sent, the field holds the shipped prompt and the disclosure reads "Prompt · default"

#### Scenario: Blank prompt
- **WHEN** a user empties the prompt and activates "Save changes"
- **THEN** the bar shows `Categorization prompt must not be empty` and the field stays empty

### Requirement: Auto Categorization Context Length Choice
The categorization "Context length" SHALL have a switch labelled "Auto" (accessible name "Auto context length"), on when `categorization_context_length_is_auto` is `true`. While it is on, "Context length" SHALL show the Auto value, read-only. Turning it off SHALL make the field editable, starting at that value. Turning it on and saving SHALL send `categorization_context_length` `null`. Below the field the card SHALL show a hint on how Auto scales.

#### Scenario: Auto shown
- **WHEN** a user opens the categorization "Advanced", the context length is Auto and 9 categories have an AI prompt
- **THEN** "Auto context length" is on, "Context length" shows `16384` read-only, and below it the page says `Auto: sized to fit your categories, about 1,800 tokens each. If it's too small, videos are checked one category at a time, which is slower.`

#### Scenario: Overriding
- **WHEN** a user turns "Auto context length" off
- **THEN** "Context length" is editable and holds `16384`

#### Scenario: Back to Auto
- **WHEN** `8192` is saved, a user turns "Auto context length" on and activates "Save changes"
- **THEN** a `PATCH /api/preferences` with only `categorization_context_length` `null` is sent, and "Context length" shows the Auto value, read-only

#### Scenario: Turned off and on without saving
- **WHEN** the context length is Auto and a user turns "Auto context length" off then on again
- **THEN** no save bar shows and no `categorization_context_length` would be sent

### Requirement: Provider Choice
While `anthropic_available` is `true`, the "Provider" card SHALL offer two option cards, set to the saved `ai_provider`:
- "Ollama", described "Runs on your own hardware. Free, private, slower on small machines."
- "Anthropic", described "Claude models in the cloud. Fast and accurate, billed per token to your API key."

Changing the choice SHALL be saved from the save bar as `ai_provider`. While `anthropic_available` is `false`, no choice SHALL show, unless `"anthropic"` is saved. Then the card SHALL say `Anthropic needs an API key. Set YARRTUBE_ANTHROPIC_API_KEY and restart yarrtube.` and offer only "Ollama".

#### Scenario: No key
- **WHEN** `anthropic_available` is `false` and the saved provider is Ollama
- **THEN** no provider choice is shown

#### Scenario: Switching to Anthropic
- **WHEN** `anthropic_available` is `true`, a user selects "Anthropic" and activates "Save changes"
- **THEN** a `PATCH /api/preferences` with only `ai_provider` `"anthropic"` is sent, the Ollama URL field is hidden and the cards show the Anthropic settings

#### Scenario: Key removed
- **WHEN** the saved provider is `"anthropic"` and `anthropic_available` is `false`
- **THEN** the card says `Anthropic needs an API key. Set YARRTUBE_ANTHROPIC_API_KEY and restart yarrtube.` and offers "Ollama"

### Requirement: Effort Field
While Anthropic is selected, the "Summaries" and "Categorization" cards SHALL each show "Effort" directly, as a segmented choice of "Low", "Medium" and "High" (accessible names "Summaries effort" and "Categorization effort"). It SHALL be filled with the card's saved effort and saved from the save bar as `anthropic_summaries_effort` or `anthropic_categorization_effort`.

#### Scenario: Changing effort
- **WHEN** Anthropic is selected and a user chooses "Low" in "Summaries effort" and activates "Save changes"
- **THEN** a `PATCH /api/preferences` with only `anthropic_summaries_effort` `"low"` is sent

### Requirement: Prompt Guide
The prompt guide SHALL list every variable the summary prompt can use, once each, written as `{name}`, with what it is replaced by. It SHALL NOT explain the prompt's sections.

#### Scenario: Variables for a section
- **WHEN** a user opens the guide
- **THEN** it lists `{part}`, `{span}` and `{summary}` among the variables, each with a description, and no section headings

#### Scenario: Every variable described
- **WHEN** a user reads the guide
- **THEN** every variable the shipped prompt uses is listed once, with a description
