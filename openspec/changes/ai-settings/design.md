# Design

## Files

Rust
- `src/domain/preferences/preferences.rs` — add AI fields + `apply(change)`; loses `Copy`.
- `src/domain/preferences/preferences_change.rs` — new: partial update (`None` keeps the stored value).
- `src/domain/preferences/summary_settings.rs` — new: the summary group of preferences.
- `src/domain/preferences/summary_model.rs` — new VO: non-blank model name ≤ 200 chars.
- `src/domain/preferences/context_length.rs` — new VO: 1024..=131072.
- `src/domain/preferences/temperature.rs` — new VO: 0.0..=2.0.
- `src/domain/preferences/summary_prompt.rs` — new VO: section/variable validation + `shipped()` (`include_str!` of `prompts/summaries/v3.md`).
- `src/domain/preferences/summary_prompt_section.rs` — new: the 9 sections, their headings and allowed variables.
- `src/domain/preferences/errors.rs` — `SavePreferencesError` → `UpdatePreferencesError` with `Invalid`.
- `src/domain/llm_server/{mod,llm_server,llm_model,ollama_url,errors}.rs` — new aggregate: a server's version + models, the URL VO (used by preferences too), `ReadLlmServerError`.
- `src/domain/services/preferences_updater.rs` — `save(Preferences)` → `update(PreferencesChange)`.
- `src/domain/services/llm_server_searcher.rs` — new: checks a server by URL.
- `src/infrastructure/repositories/sqlite_preferences_repository.rs` — `StoredPreferences` gains the new keys (missing → default; prompt stored only when `Some`).
- `src/infrastructure/repositories/ollama_llm_server_repository.rs` — new: `LlmServerRepository` port + reqwest blocking adapter (`/api/version`, `/api/tags`, `POST /api/show` per model for `capabilities`, 5 s timeout each).
- `src/infrastructure/infrastructure_container.rs` — build the Ollama adapter.
- `src/application/http/preferences/{mod,dto}.rs` — `update_preferences` becomes PATCH over a partial DTO.
- `src/application/http/ollama/{mod,dto}.rs` — new: `GET /api/ollama?url=`.
- `src/application/http/mod.rs` — `.patch(...)` instead of `.put(...)`; `/ollama` route; `llm_server_searcher` in `ApiServices`.
- `src/serve.rs` — construct `LlmServerSearcher`.
- `Dockerfile`, `.dockerignore` — `COPY prompts ./prompts` and `!prompts/**/*.md` (`*.md` is ignored today; `include_str!` needs it).

Web
- `web/src/api/types.ts` — `Preferences` gains AI fields; `PreferencesPatch`; `OllamaServer`.
- `web/src/api/client.ts` — `patchPreferences`, `fetchOllama(url)`.
- `web/src/api/queries.ts` — `useSavePreferences`/`useSaveTheme` send patches; `useOllama(url)`, `useCheckOllama()`.
- `web/src/test/helpers.tsx` — `aPreferences`, `anOllamaServer` builders.
- `web/src/lib/promptGuide.ts` — new: sections + variables + descriptions (mirrors `summary_prompt_section.rs`).
- `web/src/lib/aiSettingsForm.ts` — new: form state ↔ preferences, `changedFields(saved, form)`.
- `web/src/components/AiSettingsView.tsx` — new page; `PromptHelp.tsx` — the "?" next to "Prompt" opening the guide in a scrollable popover.
- `web/src/components/{SettingsMenu,PreferencesView,ThemeToggle}.tsx`, `web/src/App.tsx` — "AI" entry, `/ai` route, patches.

E2E
- `e2e-tests/ollama-stub.js` — new stub (`/api/version`, `/api/tags`, `/api/show`), started like `plex-stub.js`.
- `e2e-tests/helpers/fixtures.js` — reset via `PATCH` (incl. `summaries_prompt: null`).
- `e2e-tests/tests/aiSettings.spec.js` — new.

Docs: `doc/AI.md` (settings now in the app).

## Types & Signatures

```rust
// domain/llm_server
pub struct OllamaUrl(String);                       // http(s), no trailing slash
impl OllamaUrl { pub fn new(v: String) -> Result<Self, ValidationError>; pub fn as_str(&self) -> &str; }
pub struct LlmModel { pub name: String, pub size_bytes: u64, pub thinking: bool }
pub struct LlmServer { pub version: String, pub models: Vec<LlmModel> } // models sorted by name
pub enum ReadLlmServerError { Unreachable { url: OllamaUrl, source: anyhow::Error } }
// Display: "Could not reach Ollama at {url}: {source}"

// domain/preferences
pub struct SummaryModel(String);   impl SummaryModel  { pub fn new(v: String) -> Result<Self, ValidationError>; }
pub struct ContextLength(u32);     impl ContextLength { pub fn new(v: i64) -> Result<Self, ValidationError>; }
pub struct Temperature(f64);       impl Temperature   { pub fn new(v: f64) -> Result<Self, ValidationError>; }

pub enum SummaryPromptSection { System, User, Chapters, MapSystem, MapUser, MapChapters, ReduceSystem, ReduceUser, ReducePart }
impl SummaryPromptSection {
    pub const ALL: [Self; 9];
    pub fn heading(self) -> &'static str;              // "map user"
    pub fn variables(self) -> &'static [&'static str]; // ["title", "minutes", ...]
}
pub struct SummaryPrompt(String);
impl SummaryPrompt {
    pub fn new(v: String) -> Result<Self, ValidationError>;
    pub fn shipped() -> Self;                           // prompts/summaries/v3.md
    pub fn as_str(&self) -> &str;
}

pub struct SummarySettings {
    pub enabled: bool,
    pub model: SummaryModel,
    pub context_length: ContextLength,
    pub temperature: Temperature,
    pub thinking: bool,
    pub prompt: Option<SummaryPrompt>,                  // None = shipped
}
impl SummarySettings { pub fn effective_prompt(&self) -> SummaryPrompt; }

pub struct Preferences {                                // + existing fields
    pub ai_enabled: bool,
    pub ollama_url: Option<OllamaUrl>,
    pub summaries: SummarySettings,
}
impl Preferences {
    /// Errors with "Ollama URL must be set to enable AI".
    pub fn apply(self, change: PreferencesChange) -> Result<Self, ValidationError>;
}

#[derive(Default)]
pub struct PreferencesChange {
    pub categories_enabled: Option<bool>,
    pub home_show_continue_watching: Option<bool>,
    pub home_show_categories: Option<bool>,
    pub home_show_quick_watches: Option<bool>,
    pub home_show_latest: Option<bool>,
    pub theme: Option<Theme>,
    pub ai_enabled: Option<bool>,
    pub ollama_url: Option<Option<OllamaUrl>>,          // Some(None) = unset
    pub summaries_enabled: Option<bool>,
    pub summaries_model: Option<SummaryModel>,
    pub summaries_context_length: Option<ContextLength>,
    pub summaries_temperature: Option<Temperature>,
    pub summaries_thinking: Option<bool>,
    pub summaries_prompt: Option<Option<SummaryPrompt>>, // Some(None) = back to shipped
}

pub enum UpdatePreferencesError { Invalid(ValidationError), Repository(anyhow::Error) }

// domain/services
pub trait PreferencesUpdaterApi: Send + Sync {
    fn update(&self, change: PreferencesChange) -> Result<Preferences, UpdatePreferencesError>;
}
pub trait LlmServerSearcherApi: Send + Sync {
    fn get(&self, url: OllamaUrl) -> Result<LlmServer, ReadLlmServerError>;
}

// infrastructure
pub trait LlmServerRepository: Send + Sync {
    fn find(&self, url: &OllamaUrl) -> anyhow::Result<LlmServer>;
}
pub struct OllamaLlmServerRepository;                    // reqwest blocking, 5 s timeout

// application/http/preferences/dto.rs
pub struct UpdatePreferencesRequest {                   // every field #[serde(default)]
    pub categories_enabled: Option<bool>, /* … existing … */ pub theme: Option<String>,
    pub ai_enabled: Option<bool>,
    pub ollama_url: Option<String>,                     // "" = unset
    pub summaries_enabled: Option<bool>,
    pub summaries_model: Option<String>,
    pub summaries_context_length: Option<i64>,
    pub summaries_temperature: Option<f64>,
    pub summaries_thinking: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    pub summaries_prompt: Option<Option<String>>,       // absent / null / text
}
pub struct PreferencesResponse {                        // + existing fields
    pub ai_enabled: bool,
    pub ollama_url: Option<String>,
    pub summaries_enabled: bool,
    pub summaries_model: String,
    pub summaries_context_length: u32,
    pub summaries_temperature: f64,
    pub summaries_thinking: bool,
    pub summaries_prompt: String,                       // effective prompt
    pub summaries_prompt_is_default: bool,
}

// application/http/ollama/dto.rs
pub struct OllamaQuery { pub url: Option<String> }
pub struct OllamaResponse { pub version: String, pub models: Vec<OllamaModelResponse> }
pub struct OllamaModelResponse { pub name: String, pub size_bytes: u64, pub thinking: bool }
```

```ts
// web/src/api/types.ts
export interface Preferences { /* existing */ ai_enabled: boolean; ollama_url: string | null
  summaries_enabled: boolean; summaries_model: string; summaries_context_length: number
  summaries_temperature: number; summaries_thinking: boolean
  summaries_prompt: string; summaries_prompt_is_default: boolean }
export type PreferencesPatch = Partial<Omit<Preferences, 'summaries_prompt' | 'summaries_prompt_is_default' | 'ollama_url'>>
  & { ollama_url?: string; summaries_prompt?: string | null }
export interface OllamaServer { version: string; models: { name: string; size_bytes: number; thinking: boolean }[] }
// web/src/lib/promptGuide.ts
export interface PromptVariable { name: string; description: string }
export interface PromptSectionGuide { heading: string; purpose: string; variables: PromptVariable[] }
export const PROMPT_SECTIONS: PromptSectionGuide[]
// web/src/lib/aiSettingsForm.ts
export interface AiSettingsForm { ai_enabled: boolean; ollama_url: string; summaries_enabled: boolean
  summaries_model: string; summaries_context_length: string; summaries_temperature: string
  summaries_thinking: boolean; summaries_prompt: string }
export function formFrom(p: Preferences): AiSettingsForm
export function changedFields(saved: Preferences, form: AiSettingsForm): PreferencesPatch
```

## Call Stack

Save (PATCH /api/preferences, body = any subset)
1. `update_preferences(State<PreferencesUpdater>, Json<UpdatePreferencesRequest>)`
2. → `PreferencesChange` built field by field with VO constructors (`?` → 400)
3. → `run_blocking(updater.update(change))`
4. → `repository.find()` → `unwrap_or_default()` → `preferences.apply(change)` (`Invalid` → 400) → `repository.save(&updated)`
5. → `PreferencesResponse::from(updated)` (prompt = `effective_prompt()`, `is_default` = `prompt.is_none()`)

Read (GET /api/preferences): unchanged path; response adds the new fields.

Check Ollama (GET /api/ollama?url=)
1. `get_ollama(State<LlmServerSearcher>, Query<OllamaQuery>)`
2. → `OllamaUrl::new(required(query.url, MISSING_OLLAMA_URL)?)?`
3. → `run_blocking(searcher.get(url))`
4. → `repository.find(&url)` → `GET {url}/api/version` → `GET {url}/api/tags` → per model `POST {url}/api/show {"model": name}` (`thinking` = `capabilities` contains `"thinking"`) → `LlmServer` (models sorted)
5. → error → `ReadLlmServerError::Unreachable` → 502; ok → `OllamaResponse`

Web
- `AiSettingsView` → `usePreferences()` → `formFrom` → user edits → Save → `changedFields(saved, form)` → `patchPreferences` → setQueryData.
- Load with saved URL → `useOllama(saved url)` (one fetch, no retry) → models for the picker.
- Test connection → `useCheckOllama()(form.ollama_url)` → message; models replace the picker's list.
- Thinking switch → hidden when the selected model is listed with `thinking: false`; `changedFields` then omits `summaries_thinking`.
- Reset to default → `patchPreferences({ summaries_prompt: null })`.

## Test Plan

### 1. Behaviour tests

`application/http/preferences` (acceptance, real SQLite)
1. `it_should_return_the_ai_defaults_if_nothing_saved` — AI off, URL null, qwen3:4b/12288/0.3/thinking, shipped prompt, `is_default` true.
2. `it_should_change_only_the_preferences_sent` — seeded `home_show_latest=false`; patch `categories_enabled=false` → both false, rest default (replaces the old full-PUT tests).
3. `it_should_keep_everything_on_an_empty_patch`.
4. `it_should_save_the_ollama_url_without_trailing_slash`.
5. `it_should_unset_the_ollama_url_on_an_empty_string`.
6. `it_should_enable_ai_with_a_url`.
7. `it_should_fail_to_enable_ai_without_a_url` — 400, stored unchanged.
8. `it_should_fail_to_unset_the_url_while_ai_is_on`.
9. `it_should_save_the_summary_settings` — model/ctx/temperature/thinking.
10. `it_should_fail_on_an_out_of_range_temperature_and_save_nothing` — sent with `categories_enabled=false`; stored unchanged.
11. `it_should_save_an_edited_prompt` — `is_default` false.
12. `it_should_not_store_the_prompt_when_saving_other_preferences` — stored row has no prompt.
13. `it_should_return_to_the_shipped_prompt_on_null`.
14. `it_should_fail_on_an_invalid_prompt` — missing section message.
15. `it_should_save_the_theme` / `it_should_fail_on_an_unknown_theme` (kept, ported to PATCH).

`application/http/ollama` (acceptance, mockito Ollama)
16. `it_should_return_the_server_version_and_models_sorted_by_name` — each with its `thinking` capability.
17. `it_should_return_no_models_if_none_installed`.
18. `it_should_fail_if_the_url_is_missing` — 400 message.
19. `it_should_fail_if_the_url_is_not_http` — 400 message.
20. `it_should_fail_with_bad_gateway_if_ollama_is_unreachable` — 502, message prefix.

Value objects (one file each)
21. `ollama_url`: accepts http/https, strips trailing slash, rejects schemeless/other schemes/empty.
22. `summary_model`: rejects blank, > 200 chars.
23. `context_length`: bounds 1024/131072 inclusive, rejects 1023/131073.
24. `temperature`: bounds 0/2 inclusive, rejects −0.1/2.1, NaN.
25. `summary_prompt`: shipped prompt is valid; missing / unknown / repeated / empty section messages; disallowed variable message; note before first section ignored; non-variable braces ignored.

Web (Vitest)
26. `aiSettingsForm.test.ts` — `changedFields` sends only changed keys; prompt omitted when untouched; empty URL → `""`.
27. `promptGuide.test.ts` — every section of the shipped prompt (`?raw` import of `prompts/summaries/v3.md`) is in the guide, and every `{var}` it uses is listed for its section.
28. `AiSettingsView.test.tsx` — fresh defaults with summary fields disabled; enabling AI enables them; Save sends only changed fields and shows "Saved"; save error shown and form kept; Test connection success/failure messages; model list with `(not installed)`; text field when server down; Thinking switch shown for thinking models and when models unknown, replaced by `<model> doesn't support thinking` otherwise; default prompt note; Reset sends `null`.
29. `SettingsMenu.test.tsx` — "AI" entry navigates to `/ai`.
30. `PreferencesView.test.tsx`, `ThemeToggle.test.tsx` — updated to expect `PATCH` with only the changed preference.

### 2. Infrastructure tests

`sqlite_preferences_repository`
31. `it_should_find_saved_ai_preferences` — round trip incl. URL and prompt.
32. `it_should_read_missing_ai_keys_as_defaults` — row written by the old shape.
33. `it_should_find_no_prompt_if_saved_without_one`.

`ollama_llm_server_repository` (mockito)
34. `it_should_read_the_version_and_models`.
35. `it_should_read_whether_each_model_can_think` — from `/api/show` `capabilities`.
36. `it_should_fail_if_the_version_endpoint_is_not_json`.
37. `it_should_fail_if_the_server_replies_with_an_error`.

E2E (`aiSettings.spec.js`, Ollama stub)
38. Settings → AI; type stub URL, Test connection shows version + model count; enable AI + summaries, pick model, Save; reload shows saved values; reset prompt.
