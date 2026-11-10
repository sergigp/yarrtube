# Categorization: decision log

What video categorization uses and why. Newest decision first. How the eval
works: [README.md](README.md).

## In use

Nothing passes the bar yet. The app ships **`qwen3:4b` with thinking**, prompt
[`v2-batch`](../../prompts/categorization/v2-batch.md), temperature 0, one call
per video about all its pending categories (Auto `num_ctx`, 16384 for 9; output
cap 1,365 per category, at least 3072), and one call per category for a video
that call can't answer. See the two entries below.

## 2026-11-10 · batch shipped with its fallback · single calls on `v2-batch` unmeasured

The app now asks as recommended below: one call per video, and for a video
whose call is rejected (output cap, refusal, timeout) or leaves categories
without a usable answer, one call per such category, in the same run.
`--batch` does the same (rows marked `fallback`, report section "Fallbacks"),
with the app's Auto `num_ctx` and per-call cap.

**Gap: a single-category call with prompt `v2-batch` has never been measured.**
The app uses `v2-batch` for every call, fallback ones included, so there is
one prompt to edit; every per-category number so far is from `v2`. The two
differ only in wording for one category vs several. The check planned for this
change, the fallback on *1 in a Billion Checkmate?* (whose 9 categories `v2`
got right one at a time), was stopped: Ollama was shared and ran at 4 tokens/s.
It concerns about 1 video in 58. To close it: run `--batch` on an idle machine
and read the fallback rows of that video, or, for a real measure, a run one
call per category with `--prompt v2-batch` (not wired up: `--prompt` changes
the system prompt, the user message stays v2's).

The new `--batch` code was checked on synthetic rows (Auto `num_ctx`, caps,
"Fallbacks" and speed sections), not yet in a run against Ollama.

## 2026-11-09 · batch vs one call per category · batch, with a fallback

Runs, both on the tuned prompts, `qwen3:4b` digest `359d7dd4bcda`, on a
MacBook Pro: [`per category`](results/2026-11-09_1958_prompt-v2_qwen3-4b_think-on/report.md)
(prompt `v2`, `num_ctx` 4096, cap 3072),
[`batch`](results/2026-11-09_1903_prompt-v2-batch_qwen3-4b_think-on_batch/report.md)
(prompt `v2-batch`, `num_ctx` 16384, cap 12288). 58 videos: the same 2 still
have no summary.

Wrong / missed / errors. Pass needs at most 1 wrong and no errors:

| Category | Labeled | Per category | Pass | Batch | Pass |
|---|---|---|---|---|---|
| Jobava London | 5 | 0 / 1 / 2 | no | 0 / 1 / 1 | no |
| Sicilian | 7 | 0 / 0 / 6 | no | 0 / 0 / 1 | no |
| Chess gameplays | 13 | 1 / 0 / 0 | yes | 1 / 0 / 1 | no |
| Science News | 9 | 1 / 1 / 2 | no | 1 / 1 / 1 | no |
| AI news | 10 | 1 / 4 / 0 | yes | 1 / 3 / 1 | no |
| Tech | 4 | 1 / 1 / 2 | no | 0 / 2 / 1 | no |
| Politics and Economy News | 4 | 3 / 0 / 1 | no | 2 / 1 / 1 | no |
| Food for thought | 9 | 4 / 1 / 1 | no | 2 / 2 / 1 | no |
| Coding tutorials | 5 | 1 / 0 / 1 | no | 1 / 1 / 1 | no |
| **Total** | | **12 / 8 / 15** | **2** | **8 / 11 / 9** | **0** |

| | Calls | Output tokens per video | Min per video on the Air (11 tokens/s) |
|---|---|---|---|
| Per category | 522 | 10,706 | 16.2 |
| Batch | 58 | 3,580 | 5.4 |

- **Batch is 3× cheaper**: a third of the output tokens per video. Seconds on
  the Pro (64-74 tokens/s) don't carry over; the Air estimate is from tokens.
- **No bleed**: batch made no wrong answer that one call per category didn't. The
  8 wrong it made are all shared, so they come from the prompts or labels.
  Missed answers where batch reasons with another category's rule: none. The
  *$18K/Month App* miss quotes Tech's own "AI tools" exclusion.
- **No drop on later categories**: wrong plus missed by position, per category
  vs batch: 1/1, 0/0, 1/1, 2/2, 5/4, 2/2, 3/3, 5/4, 1/2. Food for thought and
  Politics, near the end, do better in batch.
- **Wrong only per category (4)**, by cause: exclusions ignored when the
  category is asked alone. *Mariana Trench nuke* and *Komarov* in Food for
  thought (science is excluded), *SpaceX shows off the ship for the Moon* in
  Politics (the "one segment" rule again), *FAANG mock interviews* in Tech.
  Batch rejected each one quoting the exclusion.
- **Missed only in batch (4) vs per category (1)**: *FAANG mock interviews* for
  Coding tutorials ("not a step-by-step tutorial"), *Contra el colapso* for Food
  for thought (read as current events), *The world's most extreme places* for
  Politics, *$18K/Month App* for Tech. Per category missed *How to Test Your
  Code* for AI news. Borderline cases each way, no pattern.
- **Errors are all the output cap, but fail differently**: per category, 15
  calls hit 3072, 8 of them chess videos asked about Jobava London or Sicilian.
  Batch hit 12288 once, on *1 in a Billion Checkmate?* (six openings in one
  stream), which loses all 9 categories of that video. Its next largest call was
  8,217 tokens (*Alapin Sicilian*), median 3,331. Both runs fail every category
  that had an error: without errors, both would pass 7 of 9, failing Politics
  and Food for thought.
- **Shared misses**: the Jobava London miss on *This rare opening...* is in both:
  the summary never names the opening, as in the first runs.

Recommendation: **batch**, falling back to one call per category for a video
whose batch call hits the cap. Batch is as good or better and a third of the
cost; chunks of a few categories would only limit what a cap hit loses, and the
fallback does that without paying for every video. What batch gives up from
the 2026-11-06 design: editing one category re-runs all 58 calls instead of
58 single-category ones, about 3× the tokens of that one category. Next: the
fallback in the eval, then Politics and Food for thought, which fail in both.

## 2026-11-07 · prompts tuned, not measured yet

Applied the changes the runs below pointed to: science explainers belong in
Science News and not Food for thought (the labels were right), tighter Politics
and Food for thought, smaller edits to Chess gameplays, Coding tutorials and AI
news, `MAX_OUTPUT` 3072, and Sicilian ticked for *1 in a Billion Checkmate?*. No
run has used them. The new cap rules out `--reuse`, so the next run is all calls.

## 2026-11-07 · first full runs · thinking leads, nothing passes

Runs: [`thinking off`](results/2026-11-06_1248_prompt-v2_qwen3-4b_think-off/report.md),
[`thinking on`](results/2026-11-06_1249_prompt-v2_qwen3-4b_think-on/report.md).
Partial: 522 of 540 calls. YouTube answered 429 for the subtitles of
`04wD8bKJZfw` (Jobava London) and `X0OperXCJMY` (London System near-miss), so
they have no summary yet; `--run` adds them. 7 of the 8 new summaries used
subtitles fetched by hand, the same `en` track `--summarize` keeps.

Wrong / missed / errors, on 58 videos. Pass needs at most 1 wrong and no errors:

| Category | Labeled | Thinking off | Pass | Thinking on | Pass |
|---|---|---|---|---|---|
| Jobava London | 5 | 4 / 0 / 0 | no | 0 / 0 / 7 | no |
| Sicilian | 6 | 4 / 0 / 0 | no | 1 / 0 / 8 | no |
| Chess gameplays | 13 | 4 / 0 / 2 | no | 3 / 0 / 2 | no |
| Science News | 9 | 0 / 7 / 0 | yes | 0 / 7 / 2 | no |
| AI news | 10 | 3 / 3 / 0 | no | 1 / 2 / 5 | no |
| Tech | 4 | 7 / 1 / 0 | no | 0 / 1 / 4 | no |
| Politics and Economy News | 4 | 5 / 0 / 0 | no | 5 / 0 / 2 | no |
| Food for thought | 9 | 38 / 0 / 0 | no | 17 / 0 / 6 | no |
| Coding tutorials | 5 | 3 / 1 / 0 | no | 2 / 0 / 1 | no |
| **Total** | | **68 / 12 / 2** | **1** | **29 / 10 / 37** | **0** |

| | Avg per call | 522 calls | Output tokens per call |
|---|---|---|---|
| Thinking off | 9.8 s | 1.4 h | 82 |
| Thinking on | 99.9 s | 14.5 h | 1,119 |

Notes in `labels.md` (all of them counted as doubtful): 15 of the 68 wrong and 3
of the 12 missed with thinking off, 6 of the 29 and 3 of the 10 with thinking on.
Without them thinking on has 23 wrong: Food for thought 16, Politics 3, one each
in Sicilian, Chess gameplays, AI news and Coding tutorials.

- **Thinking leads**: under half the wrong answers. Thinking off ignores
  exclusions: 13 chess videos went to Food for thought and 7 AI videos to Tech,
  all answered correctly with thinking.
- **Thinking off's opening recall is partly luck**: it said yes on 8 of 18 chess
  videos for Jobava London and 9 of 18 for Sicilian, calling Jobava "a variation
  of the Sicilian". One of its Jobava hits has a summary that never names the
  opening.
- **Thinking's 37 errors are the output cap**: 35 ran out of the 2048 tokens
  before answering. They are spread over all categories, so it's the cap, not one
  prompt. Input stays under 710 tokens, so a higher cap fits in `num_ctx` 4096.
- **`think: false` works here**: 82 tokens per call, against the "ignores it"
  gotcha in the summaries log. Its 2 errors are move lists looping in `reason`.
- **One Sicilian game in a stream gets caught**: both cases, each with a chapter
  naming the opening. Thin evidence. One of them, *1 in a Billion Checkmate?*
  (chapter "Grand Prix Attack vs Sicilian"), is labeled no, so it counts as wrong
  and the label looks like the mistake.
- **Broad categories catch too much**: Food for thought said yes 47 times with
  thinking off and 26 with it on, for 9 labeled. Politics also takes essays and
  videos that only touch on policy.
- **Speed**: a timing check of 10 calls underestimated: thinking fell from about
  24 to 11 tokens/s and stayed there. Heat fits, not confirmed. At 15 min per video
  with thinking, against 5 for a summary, the backlog cost needs a decision.

Thinking on's 39 wrong and missed, by cause:

| Cause | Count | Examples |
|---|---|---|
| Category prompt vs labels: science explainers | 15 | Prompts send them to Food for thought and exclude them from Science News; labels say the opposite |
| Category prompt: Food for thought too broad | 9 | "how technology works" takes tools and system design |
| Category prompt: Politics too broad | 4 | Essays on capitalism, aviation rules behind a skyscraper |
| Category prompt: chess game vs lesson | 3 | Coaching sessions, a Karpov game used as an example |
| Category prompt: AI, Tech and Coding boundaries | 5 | Running a model as a coding tutorial, *The AI bubble* |
| System prompt: "one segment" | 1 | A satellite incident in a space news show counted as Politics |
| Summary missing the information | 1 | Wayfinder's summary never says it's for AI agents |
| Label to review | 1 | *1 in a Billion Checkmate?* for Sicilian |

Thinking off adds a cause: the model ignoring the prompt's exclusions. A summary
also lost the opening in *This rare opening...*: it never names the Jobava, and
thinking hit the cap there, so it isn't in the table. The known risk is real but
rare: chapters and titles name the opening in the other cases.

Next: decide on the science labels, then tune the category prompts on a new run
with `--reuse`, with a higher output cap. Then the last check on fresh videos.

## 2026-11-06 · design, before any run

- **One call per video + category**, not all categories in one call: a video
  can fit none or several, editing one category re-runs only that category,
  and rules can't bleed between categories. Unmeasured: the eval times it.
- **Summaries are required**, and chapters are sent with them. Summaries can
  get details wrong but rarely the subject. Known risk: a stream with many
  openings may lose the one the category needs.
- **A real part of the video is enough**: one Sicilian game in a stream counts,
  a passing mention doesn't. The risk is broad categories catching too much.
- **The pass bar is at most 1 wrong video per category**, set before any run.
  Report counts, not percentages: with 5 matches one miss is already 20%.
- **Golden set**: 30 random videos with summaries in prod, 20 hand-picked for
  Jobava, Sicilian and near-misses, plus the 10 summary cases. Prod summaries
  exist only for recent channel videos, so the random part reflects those.
- **Starting settings**: temperature 0, `num_ctx` 4096, thinking compared on
  and off.
