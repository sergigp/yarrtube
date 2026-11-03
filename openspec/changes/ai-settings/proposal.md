# Proposal

## Why

Summaries (model `qwen3:4b`, prompt `prompts/summaries/v3.md`, see
`evals/summaries/decisions.md`) need somewhere to be configured before the app
can generate them: where Ollama is, which model and settings to use, and the
prompt. Users run different hardware and models, so these must be editable from
the web UI and stored by the daemon, not hardcoded.

## What Changes

- New preferences, all off/defaulted so nothing changes until the user opts in:
  - `ai_enabled` (default `false`), `ollama_url` (default unset).
  - `summaries_enabled` (default `false`), `summaries_model` (`qwen3:4b`),
    `summaries_context_length` (`12288`), `summaries_temperature` (`0.3`),
    `summaries_thinking` (`true`), `summaries_prompt` (the text of
    `prompts/summaries/v3.md`).
- Validation on save: Ollama URL must be http(s); AI can't be enabled without
  a URL; context length and temperature in range; the prompt must hold every
  section the summary flow uses and only the placeholders each section allows.
- **BREAKING** `PUT /api/preferences` is replaced by `PATCH /api/preferences`:
  only the preferences sent change. The web UI is its only client.
- New `GET /api/ollama?url=<url>`: checks an Ollama server (saved or not) and
  returns its version and installed models. Used by the "Test connection"
  button and to fill the model picker.
- New web UI page "AI" (Settings menu → AI, route `/ai`):
  - AI on/off, Ollama URL, "Test connection" (shows version or the error).
  - Summaries section: on/off, model (picked from the server's models, or typed
    when the server is unreachable), context length, temperature, thinking.
  - Prompt editor: one text area with the whole prompt document (`## section`
    headings as in `v3.md`), a guide listing each section's purpose and its
    available variables, and "Reset to default".
- The default prompt lives in code (`prompts/summaries/v3.md`, embedded at
  build time). Nothing is stored until the user saves the prompt; until then
  the page shows the default (`summaries_prompt_is_default`). Once saved, the
  text is kept across upgrades; "Reset to default" drops it (`null`).

Decisions worth recording:

- Thinking is a toggle (default on): the evals found it more accurate. Only
  some models support it (qwen3, deepseek-r1, magistral, gpt-oss), and Ollama
  rejects `think` for the others, so the Ollama check reports each model's
  `thinking` capability (from `/api/show`) and the switch shows only for
  models that have it. The generation change must send `think` only when the
  model supports it, and still strip everything up to `</think>` (`qwen3:4b`
  thinks even when off). gpt-oss's `low`/`medium`/`high` levels are out of
  scope: it's treated as on/off. Context length and temperature apply to
  every Ollama model. Output cap (`num_predict`) and request timeout stay
  internal limits.

Out of scope: generating, storing or showing summaries (next change), Ollama
reachability in the status endpoint, categorization and Q&A settings.

## Capabilities

### New Capabilities
- `ollama-connection`: checking an Ollama server and listing its models.
- `ai-settings-ui`: the web UI page for the AI preferences.

### Modified Capabilities
- `preferences`: new AI and summary preferences with their defaults and
  validation; saving becomes a partial `PATCH`.
- `web-ui`: the header settings menu gains an "AI" entry.

## Impact

- Rust: `domain/preferences` (new value objects, `Preferences` no longer
  `Copy`), preferences handler/DTOs, stored preferences JSON (no migration:
  missing keys read as defaults), new Ollama port + adapter (reqwest, already a
  dependency), new `domain/llm_server` aggregate + checker service, router.
- Web: `api/types.ts`, `api/client.ts`, `api/queries.ts` (save → patch),
  `PreferencesView`, `ThemeToggle`, `SettingsMenu`, new `AiSettingsView`, a
  prompt guide in `src/lib/`.
- E2E: fixture resets preferences via `PATCH`; new Ollama stub server
  (like `plex-stub.js`) and an AI settings spec.
- `GET /api/preferences` responses grow by the prompt (~4 KB).
- Docs: `doc/AI.md` (settings now in the app), `doc/INSTALLATION.md` if it
  documents preferences.
