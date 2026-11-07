# Categorization: decision log

What video categorization uses and why. Newest decision first. How the eval
works: [README.md](README.md).

## In use

Nothing yet: no settings pass the bar. Leading: **`qwen3:4b` with thinking**,
prompt [`v2`](../../prompts/categorization/v2.md), temperature 0, `num_ctx` 4096,
a higher output cap, once the [category prompts](category-prompts.md) are
tuned.

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
