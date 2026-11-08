# AI features (Ollama)

Working notes for adding local LLM features to yarrtube. Brainstorm, not a
spec: decisions here feed OpenSpec changes later.

## Goals

1. **Summary**: a short summary per video from its data (title, length,
   transcript), shown in the player instead of the spammy YouTube
   description.
2. **Auto-categorization**: when a new video is reconciled, decide which
   categories it belongs to. Each category gets a user-written prompt, and the
   video is evaluated against each one.
3. **Ask the video**: a chat in the player that answers questions about the
   video using its subtitles.

## Decisions

- Ollama runs on a MacBook Air M2, 8GB RAM, always plugged in, used as a home
  lab server. yarrtube (Docker, NAS) talks to it over the LAN.
- Summaries are written in the **video's own language**.
- Categorization is **automatic** (assigned, not suggested).
- Build order: summary → categorization → ask the video. Each step reuses
  the previous one.
- Summary model, prompt and the evidence behind them: [`evals/summaries/decisions.md`](../evals/summaries/decisions.md).
- Prompts live in `prompts/<feature>/<version>.md`, one file per version, never
  edited once evaluated: a change is a new version. The evals and the app read
  the same file.
- Settings live in the app (Settings → AI, stored with the other preferences):
  AI on/off, the Ollama URL (with a connection test that lists the models),
  and for summaries the model, context length, temperature, thinking (shown
  only for models whose Ollama capabilities include `thinking`, since Ollama
  rejects `think` for the rest) and the prompt. The default prompt is the
  shipped version, embedded at build time (`SummaryPrompt::shipped()`):
  shipping a new version is changing that one path. Users who never saved a
  prompt get it on upgrade; a saved prompt is kept until "Reset to default".
- On the AI page, categorization has its own switch, model, context length
  (4096), temperature (0), thinking (on) and wrapping prompt
  (`prompts/categorization/v2.md`, plain text with no sections or variables,
  saved like the summary prompt): the values `evals/categories` runs with,
  still under evaluation. Each category has an optional AI prompt (`ai_prompt`)
  describing the videos it should hold, written in the category dialog, when
  adding it or under "Edit settings", while AI, categories and categorization
  are all on, and each source a `categorization_enabled` opt-in. What the model
  gets is the wrapping prompt plus the video's details and the category's
  prompt, see [How a video is categorized](#how-a-video-is-categorized).

## Starting point in the codebase

- `video-subtitles` already stores a plain-text transcript per video
  (state Missing / Fetched / Unavailable), explicitly so it can later be
  summarised.
- `video-categories`: a video belongs to its source's categories plus every
  category with a `yes` verdict for it.

## Constraints

### Hardware

- 8GB unified memory, macOS uses ~3–4GB, so about 4GB is left for the model.
- That means small models (1–4B parameters), quantized (compressed, Q4), e.g.
  Qwen3 4B, Gemma 3 4B, Llama 3.2 3B, Phi-4-mini. Good at "summarize this" and
  "is X about Y?", weaker at nuance, more prone to making things up.
- Context window (how much text the model reads at once) is measured in
  tokens, ~¾ of a word. A 30 min transcript ≈ 7k tokens, 1 h ≈ 15k. Longer
  context costs more RAM, so long videos won't fit in one call on 8GB.
- Rough speed: reading the input runs at a few hundred tokens/s, writing at
  ~15–30 tokens/s. A summary takes about 1 min without thinking; with
  `qwen3:4b` and thinking on, the default, the evals average ~5 min. Fine for
  background work, but a backlog of 2,000 videos is about a week.

### Topology

- Ollama must listen on the network (`OLLAMA_HOST=0.0.0.0`).
- The laptop is always on, but it's still a separate machine: it can reboot,
  update, or be busy. LLM work goes through background tasks that retry
  later; the rest of the app never depends on Ollama being up. Show
  "Ollama reachable?" in status.
- Summaries use Ollama's **native API** (`POST /api/chat`): the
  OpenAI-compatible one (`/v1/chat/completions`) can't set `num_ctx`, and
  Ollama truncates input silently at its small default.
- Summaries send `truncate: false` and `shift: false`. Otherwise Ollama cuts an
  oversized prompt to about half the context (0.40.13) and reports only what it
  kept, so nothing shows the summary covers a fragment. Refused, it answers
  400 at once with the exact token count.

### How a summary is generated

- The video sync (`VideoReconciler`) schedules a `summarize_video` task for a
  downloaded video with fetched subtitles, when AI and summaries are on and
  its channel or playlist opted in. That backfills a source's stored videos.
  Tasks run one at a time in their own lane, so a backlog doesn't hold other
  lanes back. Like any running task, a summary does hold back an exclusive
  task (`update_ytdlp`, hourly), and whatever waits behind it, until it ends.
- The task checks all that again when it runs, then fills the prompt with the
  video's title, duration, language, chapters and transcript. Timed lines come
  from the `.srt` on disk: the stored transcript has no timings.
- A transcript over ~73% of the context length is cut into parts, at chapter
  starts when it can be, and summarized by map-reduce. Size is estimated at 4.3
  characters per token for English, 1.5 for Japanese, Chinese and Korean, and
  3.4 otherwise. Answers that don't fit one reduce are reduced in groups first.
- Ollama down, or the model not installed, skips the run and changes nothing:
  a later sync tries again.
- Any other failure fails the task, and the queue retries it with its usual
  backoff. When the retries run out, a rejected call (ran out of context,
  empty, refused as too long, past the 30-minute timeout) or an unreadable
  transcript marks the video's summary `UNAVAILABLE`, while a server fault
  (connection error, 5xx, 404) leaves it for a later sync.
- Changing the Ollama URL or any summary setting puts every `UNAVAILABLE` video
  back in line: the new settings may work. A stored summary is never
  regenerated.
- Context length is the user's call. A low one leaves little room to think, so
  summaries will fail, each logged at error level with its reason, until it is
  raised.
- Everything is logged at info level: each call sent, with the video's id and
  title but not the prompt, each answer, how long each call and the whole
  summary took, and the video's length. Failures are logged at error level.

### How a video is categorized

- Each (video, category) pair gets at most one **verdict**: `yes`, `no` or
  `unavailable`, stored in `video_category_verdicts` with the model, its
  one-sentence reason and when. A pair with a verdict is never asked again,
  so reconcile passes don't repeat hours of calls.
- A video's categories are its source's categories plus every category with a
  `yes` verdict. Plex labels follow through `video_categories_changed`.
- The video sync schedules a `categorize_video` task when AI, categories and
  categorization are on, the video's channel or playlist has
  `categorization_enabled`, its summary is Generated, and a category is
  **pending**: it has an AI prompt, the source isn't a member of it, and the
  video has no verdict for it. The video is categorized from its summary, so
  its source needs summaries on too.
- The task runs in the summaries lane, so one Ollama call runs at a time. It
  re-checks those conditions, reads the pending categories again, and asks
  once per category: the saved (or shipped) categorization prompt as system
  message; the title, channel or playlist name, length, chapters, summary,
  category name and AI prompt as user message. Temperature 0, context 4096,
  at most 3072 output tokens (stops runaway thinking), the answer's JSON
  schema as `format`, and thinking when the model supports it. Each verdict is
  stored as it comes, unless meanwhile its category was deleted, its video
  removed, the user set that pair by hand, or the categorization settings or
  the category's AI prompt changed: then the answer is dropped and the pair
  stays pending, asked again with the new prompt.
- The answer is read after the last `</think>`: a JSON object with a boolean
  `belongs` and a `reason`.
- A rejected call (unusable answer, output limit hit, refused, past the
  30-minute timeout) fails the task once every pending category was tried; the
  retry asks only the ones still pending. On the last attempt each rejected
  category gets an `unavailable` verdict. Ollama down, or the model not
  installed, stops the run with a warning and leaves the rest pending for a
  later sync.
- Saving a new Ollama URL, categorization model or prompt deletes every
  `unavailable` verdict; a new AI prompt for a category deletes that
  category's. `yes` and `no` verdicts are final: editing or clearing a
  category's prompt, turning a source's categorization off, or turning AI off
  keeps them, and the categories they give.
- Deleting a category deletes its verdicts; removing a video, or deleting its
  source, deletes the video's.
- A verdict is **decided by** the AI or the user. The user sets one per pair
  from a video's **Edit categories…** dialog (`PUT
  /api/videos/{record_id}/categories/{category_id}`), replacing the AI's; a
  category the video's source is a member of can't be set (409). A pair with
  a user verdict is never pending, so the AI never asks about it.
- **Re-running** deletes AI verdicts (`yes`, `no` and `unavailable`) and keeps
  user ones, then recomputes the categories of the videos that had one, in one
  `video_categories_changed`: per video (`DELETE /api/videos/{record_id}/ai-verdicts`,
  then its sync), per channel or playlist (`DELETE
  /api/{channels,playlists}/{id}/ai-verdicts`, then its reconcile pass) and per
  category (`DELETE /api/categories/{id}/ai-verdicts`, then each affected
  video's sync). With categorization off nothing is asked again, so it just
  clears: the video dialog and the source menu then read **Clear AI
  categories** (`GET /api/videos/{record_id}/categories` reports the source's
  `categorization_enabled`). **Re-evaluate** in a category's settings is
  disabled while its prompt has unsaved edits, since the AI asks with the
  saved one. `GET /api/categories` reports each category's `ai_verdict_count`.
- Each verdict is logged at info level with the video, category, verdict,
  reason, tokens and duration; skips at info, rejected calls at error.

### How a summary is shown

- The video detail pane shows the whole summary under the meta line, labelled
  "AI summary", in playlist, channel and category views. Nothing is shown for a
  video without one, whether it's waiting or given up on.
- The playlist, channel and category listings return it as `summary`.
  `VideoSearcher` decides per listing, so the API never sends a hidden summary:
  it's shown only while AI is on, summaries are on and the video's own source
  has `summaries_enabled`.
- A hidden summary stays stored and comes back when all three hold again, with
  no call to Ollama.
- Each source's copy of a video follows that source's toggle. A category lists
  the channel copy when there is one, so the channel's toggle decides there.
- Saving preferences refetches the video lists, even off screen, so going back
  to one doesn't first show a summary that was turned off.
- The NFO `<plot>` keeps the YouTube description.

## Glossary

- **Prompt / system prompt**: the instructions plus the data sent to the
  model. The system prompt is the fixed part ("you are a summarizer, rules:
  …").
- **Temperature**: how random the output is. Low (0–0.3) for classification
  and summaries.
- **Structured output**: forcing the model to reply as JSON matching a schema
  (Ollama's `format`). Essential for classification: parse
  `{"match": true, "reason": "…"}` instead of free text.
- **Hallucination**: the model confidently stating something not in the
  input.
- **Embeddings**: text turned into a vector of numbers; similar meaning gives
  nearby vectors. Cheap and fast, done by a separate small model (e.g.
  `nomic-embed-text`, ~300MB).
- **RAG (Retrieval-Augmented Generation)**: instead of giving the model the
  whole document, find the most relevant chunks (usually via embeddings) and
  give it only those. Needed when the data doesn't fit in the context.
- **Chunking / map-reduce**: split long text, process each piece, then
  combine the results. How a small model summarizes a 2-hour video.
- **Prompt injection**: input text that contains instructions ("ignore
  previous instructions…"). YouTube descriptions and subtitles are untrusted
  input. Low risk here (worst case a weird summary), but keep it in mind.
- **Evals**: tests for LLM behavior, see [Evals](#evals).

## Feature A: Summary

- Input: title + transcript + duration. Skip the description, or use only its
  chapter list: spam in, spam out.
- Runs in the background once a video's subtitles are Fetched.
- Written in the video's language (the transcript's recorded language). Small
  models are best in English and reasonable in Spanish; check quality for
  other languages in evals.
- Stored in the DB with **model name + prompt version**, so changing either
  can trigger regeneration.
- Long transcripts: summarize chunks, then summarize the summaries.
- Use the video's **original YouTube title**: the prompt uses it to fix
  caption misspellings, so a translated or renamed title misleads the model.
- Strip a leading "The video…"/"El vídeo…" in code: small models ignore that
  rule however it's phrased.
- Videos with no subtitles (Unavailable): no summary. Title-only summaries are
  junk.
- The summary is also the cheap input for feature B.

## Feature B: Auto-categorization

- It's classification, what small models do best when kept simple.
- One prompt per category, one yes/no call per category per video. Simple,
  independent, easy to debug. N categories means N calls per video; with short
  inputs (title + summary, not the full transcript) each is a few seconds.
- Output via structured output, including a short `reason`: helps debugging
  and makes assignments explainable in the UI.
- Possible complement: embed the category prompt and the video summary and
  compare with cosine similarity. Near-free but fuzzy, needs a threshold.
  Hybrid: embeddings shortlist, LLM confirms.
- Automatic assignment means precision matters most (a wrong category is
  visible noise); measure it before turning it on.
- Decided: the AI adds to the source's categories; it never removes one. See
  [How a video is categorized](#how-a-video-is-categorized).

## Feature C: Ask the video

- Interactive, so speed matters.
- Two approaches:
  1. **Whole transcript in context**: simplest, works for short videos.
  2. **RAG**: split subtitles into ~30–60s chunks **keeping timestamps**,
     embed them once, store them. Per question, embed it, take the top 5
     chunks, answer from those.
- Answers cite timestamps, which become **clickable seek links in the
  player**. RAG gives this cleanly.
- Storage: ~100 chunks per video, so plain cosine similarity in Rust over
  vectors in SQLite is enough. No vector DB needed (`sqlite-vec` exists if
  ever wanted).
- Needs streaming responses (tokens shown as they're generated) and chat
  history handling.

## Evals

Normal tests check deterministic code. LLM output is fuzzy and changes when
the prompt or model changes. **Evals are a fixed set of inputs with expected
results, scored automatically**, so "I tweaked the prompt" becomes
measurable.

- **Categories** (easiest): existing data is already labeled. Every video
  whose source is in a category is an example. Run the classifier on ~200 of
  them and measure:
  - **Precision**: of the videos it said were Chess, how many really are.
  - **Recall**: of the real Chess videos, how many it found.

  Compare models (Qwen vs Gemma) and prompt versions with real numbers.
- **Summaries**: built, see [`evals/summaries/`](../evals/summaries/README.md).
- **Q&A**: ~20 questions with known answers and expected timestamps. Check
  retrieval (did it find the right chunk?) and answer quality separately; a
  key RAG lesson.

Evals run on demand (CLI command or dedicated suite), not in CI: slow and
they need Ollama.

How evals work here, for every feature: [`evals/README.md`](../evals/README.md).

## Open questions

- Backlog: summarize and categorize every stored video, or only new or
  unwatched ones?
- Translated titles: does yarrtube store a localized title instead of the
  original? Check before the summary feature relies on it.
