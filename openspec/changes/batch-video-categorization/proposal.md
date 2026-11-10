# Proposal

## Why

Categorization asks the model once per video and category: about 16 minutes
of a MacBook Air per video with 9 categories. The 2026-11-09 eval
(`evals/categories/decisions.md`, "batch vs one call per category") measured
one call per video about every category: as good or better (8 wrong / 11
missed / 9 errors against 12 / 8 / 15), no rules bleeding between categories,
a third of the output tokens (~5.4 min per video). Its only new failure: a
video whose one call hits the output cap loses all its categories at once,
which a per-category fallback for that video recovers.

## What Changes

- A categorization asks about all its pending categories in **one call**
  (a batch), with one JSON answer keyed by category name. A video with one
  pending category gets a batch of one, which is today's single call.
- **Fallback**: when a batch of several categories is rejected (output cap,
  refusal, timeout, unusable answer), or answers usably for only some of
  them, each category left without a verdict is asked again in its own
  batch of one, in the same run. Only what those single calls reject goes
  through today's retry / `unavailable` rules.
- The shipped categorization prompt becomes `prompts/categorization/v2-batch.md`
  for every call, single ones included. A saved custom prompt is kept as is.
- **Output cap scales** with the categories asked: 1,365 tokens each, at
  least 3,072, at most the context length.
- **Timeout scales** with the cap: the cap at 5 tokens a second, at least
  the current 30 minutes (41 min for 9 categories), so a long batch on slow
  hardware isn't cut short. Summaries keep 30 minutes.
- **BREAKING (preferences API)**: `categorization_context_length` gets an
  **Auto** mode, the new default: 1,820 tokens per category with an AI
  prompt, rounded up to a multiple of 1,024, at least 4,096 (16,384 for 9).
  Saving a number overrides it; saving `null` goes back to Auto. A new
  read-only `categorization_context_length_is_auto` tells which. A stored
  `4096` (the old default, written by every earlier save) reads as Auto.
- The AI settings page gets an "Auto" choice for the categorization context
  length, showing the computed value, with a hint on how it scales.
- Logs: one info line per call (categories asked, tokens in/out, duration,
  whether it is a fallback), one per verdict.
- **Eval**: `evals/categories/eval.py --batch` gets the same fallback, the
  app's cap and Auto context, and reports fallback calls and their tokens
  separately.
- Unchanged: editing a category's prompt or the system prompt keeps every
  `yes`/`no` verdict; new prompts apply to future videos and to pairs still
  pending (never asked, or `unavailable` and reset).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `video-categorization`: one call per categorization for all pending
  categories, the in-run fallback, the batch request (user message, schema,
  cap scaling with the categories asked, context length), answer read per
  category, logging per call and per verdict.
- `preferences`: `categorization_context_length` Auto mode, its default, the
  `_is_auto` flag, `null` to reset, the old stored `4096` read as Auto, and
  the shipped categorization prompt now `v2-batch`.
- `ai-settings-ui`: the categorization "Context length" Auto choice with its
  computed value and hint.

## Impact

- Rust: `domain/video_category/categorization_question.rs` (N categories,
  schema keyed by name, per-category answer parsing),
  `domain/services/video_categorizer.rs` (batch then fallback),
  `domain/preferences/` (Auto context length, shipped prompt),
  `llm_server::LlmReply` (output tokens for the log), the Ollama adapter,
  the SQLite preferences repository, the preferences HTTP DTO, the
  preferences searcher (needs the count of categories with an AI prompt).
- Web: `api/types.ts`, `lib/aiSettingsForm.ts`, `components/AiSettingsView.tsx`
  and their tests and fixture builder.
- Prompts: `prompts/categorization/README.md` (v2-batch shipped).
- Eval: `evals/categories/eval.py`, its README and `decisions.md`.
- Docs: `doc/AI.md`, `doc/CATEGORIES.md`.
- No database migration: preferences are a JSON blob, read leniently.
