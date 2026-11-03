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

## Starting point in the codebase

- `video-subtitles` already stores a plain-text transcript per video
  (state Missing / Fetched / Unavailable), explicitly so it can later be
  summarised.
- `video-categories` already supports per-video categories: a video whose
  source is in no category keeps categories set individually, "none until a
  classifier sets any". Source categories take precedence.

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
  ~15–30 tokens/s. Summarizing a 10k-token transcript ≈ 1 min. Fine for
  background work; a backlog of 2,000 videos is a night of work.

### Topology

- Ollama must listen on the network (`OLLAMA_HOST=0.0.0.0`).
- The laptop is always on, but it's still a separate machine: it can reboot,
  update, or be busy. LLM work goes through background tasks that retry
  later; the rest of the app never depends on Ollama being up. Show
  "Ollama reachable?" in status.
- Use the **OpenAI-compatible API** Ollama exposes (`/v1/chat/completions`,
  `/v1/embeddings`), so moving to a bigger box or a cloud provider later is a
  URL and model change.

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
- Open: precedence with source categories. Today a source's categories win
  over per-video ones. Should the classifier run only for videos whose source
  is in no category, or add to them?

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
- Categorization precedence vs source categories (see Feature B).
- Translated titles: does yarrtube store a localized title instead of the
  original? Check before the summary feature relies on it.
