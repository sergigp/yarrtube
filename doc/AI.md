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

- Ollama runs on a MacBook Air M1, 8GB RAM, always plugged in, used as a home
  lab server. yarrtube (Docker, NAS) talks to it over the LAN.
- Summaries are written in the **video's own language**.
- Categorization is **automatic** (assigned, not suggested).
- Build order: summary → categorization → ask the video. Each step reuses
  the previous one.

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
- **Summaries** (no single right answer):
  - a side-by-side page to rate them by eye;
  - **LLM-as-judge**: a stronger model scores each summary against a rubric
    (faithful? no spam? right language? length?), possibly a cloud model only
    during evals;
  - cheap automatic checks: length, language, no URLs or "subscribe".
- **Q&A**: ~20 questions with known answers and expected timestamps. Check
  retrieval (did it find the right chunk?) and answer quality separately; a
  key RAG lesson.

Evals run on demand (CLI command or dedicated suite), not in CI: slow and
they need Ollama.

## Findings: first summary benchmark (2026-10-30)

Setup: MacBook Air M2 8GB (fanless), Ollama 0.40.13, 9 real transcripts
(7–58 min, es/en, mostly auto-generated captions), parsed like
`TranscriptText::from_srt`. One prompt (3–5 sentences, video's language,
facts only, no spam), temperature 0.3, `num_ctx` 12288. A 10th, 3 h
transcript (~40k tokens) was left out: it needs chunking.

### Memory / context

| Model | num_ctx | RAM | Placement |
|---|---|---|---|
| qwen3:4b | 8k | 3.8GB | 100% GPU |
| qwen3:4b | 12k | 4.5GB | 100% GPU |
| qwen3:4b | 16k | 5.4GB | 16% CPU (slow) |
| gemma3:4b | 4k–16k | ~2.4GB | ~10% CPU always |

- qwen3 ceiling is ~12k tokens per call: videos up to ~45–60 min fit; longer
  ones need map-reduce.
- Transcripts run ~1.3–1.6 tokens per word. 30 min ≈ 8k tokens.
- Ollama's default context is small and it truncates silently: always set
  `num_ctx`, and compare `prompt_eval_count` with the expected size.

### Speed per video

| Model | Input tok/s | Output tok/s | Output tokens | Wall time |
|---|---|---|---|---|
| gemma3:4b | 155–187 | 14–17 | 100–180 | 23–60 s |
| qwen3:4b-instruct | 112–203 | 8–13 | 160–290 | 38–110 s |
| qwen3:4b (thinking) | 136–203 | 11–19 | 600–2,560 | 73–243 s |

- Reading the transcript dominates; time grows with length.
- `qwen3:4b` defaults to thinking mode and **ignored `think: false`**,
  writing its reasoning inside the answer. Either strip `<think>…</think>`
  or use `qwen3:4b-instruct` (no thinking).
- The qwen3-instruct run came last, after ~40 min of continuous load, and
  was slower across the board. Suspected thermal throttling (fanless Air),
  not confirmed. Matters for backlog runs.
- Rough backlog throughput: gemma3 ~80 videos/hour, qwen3-instruct ~50,
  qwen3 thinking ~25.

### Quality (read by hand)

- All three wrote in the video's language every time, including a video
  with an English title and Spanish audio. Use the transcript's language,
  not the title's.
- None followed "do not start with 'The video'". Small models ignore
  negative style rules; fix in post-processing or with an example.
- **Chess stream misread by all three**: the video has three games (two
  wins, then a draw). Every model summarized it as a single game:
  qwen3-instruct described game 2 (a win), the others game 3 (a draw).
  First read as a qwen3-instruct hallucination; drafting the eval facts
  showed otherwise. Lesson: checking by eye against one quote misleads.
- qwen3-instruct turned "almost 100 unions" into "more than 100"; thinking
  qwen3 kept "casi 100".
- gemma3: shortest, most consistent, slightly generic; carried a caption
  error ("Cloud Code" for "Claude Code").
- qwen3-instruct: richest and most specific, always at the 5-sentence
  upper bound, but inflated a number (above).
- qwen3 (thinking): most accurate, but 3–5× slower.
- Chess commentary is the hardest content: moves read out loud, several
  games in one video, the result buried in speech.

### Long video: map-reduce (3 h, ~40k tokens)

Split into chunks of ~30k chars (~7.5k tokens) with 10 lines of overlap
(6 chunks, the last one tiny), each summarized in 4–6 sentences with its time
span, then a final summary from the ordered part summaries.

Actual segments: Greenland 0–34 min, Switzerland 34–60, Saudi Arabia (NEOM)
62–94, Finland 94–146, Cyprus 146–180.

| Chunk | qwen3:4b (thinking) | gemma3:4b |
|---|---|---|
| 0:00–0:39 | Greenland ✓ | Switzerland ✗ (last 5 min only) |
| 0:38–1:15 | Switzerland ✓ | NEOM ✗ (last 13 min only) |
| 1:15–1:52 | Saudi Arabia ✓ | Finland ✗ (last 18 min only) |
| 1:52–2:25 | Finland ✓ | Finland ✓ |
| 2:25–2:58 | Finland + Cyprus ✓ | Cyprus ✓ |
| Final | all 5 places ✓ | **Greenland missing** |
| Total time | 8.8 min | 4.8 min |

- **gemma3 summarizes the end of long inputs**: on ~8k-token chunks it
  described mostly the last minutes and ignored the rest, losing Greenland (a
  third of the first chunk's hour) entirely. Likely related to its
  sliding-window attention (most layers only see nearby tokens); not
  confirmed. The 9 single-call videos were 2–9k tokens, so their gemma
  summaries may share this bias.
- gemma3 also prefixed 3 of 6 part summaries with "Okay, here's a summary…";
  the reduce step hid it.
- qwen3 (thinking) covered every segment in order and the final summary
  named all five places.
- The reduce step works: ~1.2k tokens in, 15–35 s.
- Map-reduce makes long videos predictable: ~1.5 min per 35 min of video
  with qwen3.

### Takeaways

- 9 videos read by eye can't pick a winner; it shows why evals matter. Next
  step: a fact-check eval (a few verifiable facts per video, e.g. who won)
  and LLM-as-judge for faithfulness.
- After the long-video test, leaning **qwen3:4b with thinking**: the only
  model with no factual slips found so far (though it also missed that the
  chess stream had three games), and faithful across long inputs.
  gemma3:4b is 2× faster but drops the start of long inputs. Confirm with
  evals.

## Open questions

- Backlog: summarize and categorize every stored video, or only new or
  unwatched ones?
- Categorization precedence vs source categories (see Feature B).
- Which model: pick from a first eval round on real transcripts.
