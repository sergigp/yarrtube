## Why

The categorization eval produced a real system prompt and the settings it ran with, but the app still ships the placeholder prompt and only exposes the prompt under "Advanced". Defining a category also takes two dialogs in a row, the only multi-step form in the app, and gives no help writing the prompt that decides what the category catches.

## What Changes

- Ship `prompts/categorization/v2.md` as the default categorization prompt. Saved prompts are kept.
- New preferences `categorization_context_length` (default 4096), `categorization_temperature` (default 0) and `categorization_thinking` (default on), the values the eval uses, shown under the Categorization section's "Advanced".
- One category dialog for creating and editing, with name, color, AI prompt (while AI, categories and categorization are on) and sources, replacing the "Add category" dialog and the follow-up settings dialog.
- `POST /api/categories` also takes `ai_prompt`, `channels` and `playlists`, so a category and its sources are stored together.
- A "?" next to the AI prompt opens a guide showing a good and a bad prompt. It is a click-to-open popover so it works on touch screens.
- **BREAKING** for the UI: the "Change color" action and its live header preview are removed; the color is edited in the category dialog. Rename in place stays.

Out of scope: evaluating videos, calling the model and assigning videos by AI. Nothing reads the categorization settings yet, so the max output cap is decided with that work.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `preferences`: categorization defaults, the shipped prompt version and the three new settings.
- `ai-settings-ui`: the new settings under the categorization "Advanced", and the "Thinking" rule applied per section.
- `category-crud`: creating a category with an AI prompt and sources.
- `web-ui`: one category dialog for create and edit, the prompt guide, and "Change color" removed from the category view.

## Impact

- Rust: `domain/preferences`, `domain/services/category_creator.rs`, `domain/category/errors.rs`, `application/http/{preferences,categories}`, `sqlite_preferences_repository.rs`, `serve.rs`.
- Stored preferences are JSON, so no migration. Preferences never saved read the new defaults.
- Web: `AiSettingsView`, `aiSettingsForm`, `AddCategoryDialog` (removed), `CategorySettingsDialog` (becomes `CategoryDialog`), `CategoryDetail`, `App`, API client and types, test helpers.
- `e2e-tests/tests/{categories,aiSettings}.spec.js`.
- Docs: `doc/AI.md`, `doc/CATEGORIES.md`, `prompts/categorization/README.md`.
