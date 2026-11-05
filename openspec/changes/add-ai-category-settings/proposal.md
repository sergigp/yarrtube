## Why

AI categories will put videos into categories by asking a model whether each video fits a description of the category. Before that evaluation can exist, the user needs somewhere to turn it on, pick a model, edit the wrapping prompt, describe each category, and choose which sources take part. This change adds those settings only.

## What Changes

- AI page: a "Categorization" section with an enable switch, a model picker and, under "Advanced", an editable wrapping prompt with a shipped default, reset included, working like the summary prompt but as a single plain text with no sections or variables.
- New preferences `categorization_enabled`, `categorization_model` and `categorization_prompt` (+ `categorization_prompt_is_default`).
- Categories get an optional AI prompt describing the videos they should hold, stored in the database and returned by the categories API.
- The category action and dialog "Manage sources" become "Edit settings", titled "Edit <name> settings"; the dialog gains an "AI prompt" text area, shown only while AI categorization is available.
- Channels and playlists get a `categorization_enabled` setting, off by default, gated like `summaries_enabled`. **BREAKING** for API clients: creating a source now requires it.
- Add and edit source dialogs group "Summarize videos" and a new "Categorize videos" checkbox in an "AI" section.

Out of scope: evaluating videos, calling the model, assigning videos to categories. Nothing reads these settings yet.

## Capabilities

### New Capabilities
- `source-categorization`: the per-source opt-in to AI categorization, its gate on the preferences, and its place in the API.

### Modified Capabilities
- `channel-crud`: the settings update takes `categorization_enabled`.
- `playlist-crud`: the settings update takes `categorization_enabled`.
- `preferences`: categorization preferences, their defaults, validation and saved-or-shipped prompt.
- `ai-settings-ui`: the Categorization section of the AI page; "Model" picker shared by both sections.
- `category-crud`: categories hold an optional AI prompt, set through the update endpoint and returned when listing.
- `web-ui`: "Manage sources" renamed to "Edit settings" with the AI prompt field; "AI" section with "Categorize videos" in the source dialogs.

## Impact

- Migration: `categorization_enabled` on `channels` and `playlists`, `ai_prompt` on `categories`.
- Rust: `domain/preferences`, `domain/category`, `domain/channel`, `domain/playlist`, their creators/updaters/view searchers, HTTP DTOs and handlers, SQLite repositories.
- New shipped prompt: `prompts/categorization/v1.md`.
- Web: `AiSettingsView`, `ManageSourcesDialog` (renamed), the four source dialogs, `SummariesField` (replaced), API types, test helpers.
- `e2e-tests` seed helpers must send `categorization_enabled`.
- Docs: `doc/AI.md`, `doc/CATEGORIES.md`.
