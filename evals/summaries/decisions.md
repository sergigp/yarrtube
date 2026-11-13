# Summaries: decision log

What the video summary feature uses, and the evidence behind it. Newest
decision first. How the eval works and how to run it: [README.md](README.md).

Keep a run in `results/` only while this log cites it; delete the
rest (git history keeps them).

## In use

**`qwen3:4b` (thinking) with prompt [`v3`](../../prompts/summaries/v3.md)**,
temperature 0.3, `num_ctx` 12288, map-reduce above ~9k input tokens
(chunked at chapter boundaries when the video has chapters).

Known weaknesses: videos with several parts (the chess stream with three
games is still summarized wrong); opens with "The video…" now and then
(strip it in code). About 5 min per video on the M2 Air, ~12 videos an hour.

**On Anthropic (`add-anthropic-provider`): `claude-haiku-5-5` at effort
`low`**, same prompt v3, `max_tokens` 16000, input cut for a 131072-token
context (every golden-set video fits one call). See the entry below.

## 2026-11-13 · Anthropic · claude-haiku-5-5, effort low

Runs: [`effort low`](results/2026-11-13_1041_prompt-v3_anthropic-effort-low/report-claude-sonnet.md),
[`effort medium`](results/2026-11-13_1041_prompt-v3_anthropic-effort-medium/report-claude-sonnet.md),
prompt v3, judged by Sonnet subagents as before, compared with the
[`qwen3:4b` v3 run](results/2026-11-03_prompt-v3/report-claude-sonnet.md).

| Model | Effort | Must-haves | Facts | Contradicted | Major / minor unfaithful | Fully faithful | Avg time | Cost (10 videos) |
|---|---|---|---|---|---|---|---|---|
| qwen3:4b | (thinking) | 19/23 | 33/50 | 2/50 | 4 / 14 | 3/10 | 287 s | free |
| **claude-haiku-5-5** | **low** | **22/23** | **46/50** | **0/50** | **0 / 6** | **6/10** | 2 s | $0.016 |
| claude-haiku-5-5 | medium | 20/23 | 41/50 | 0/50 | 0 / 6 | 6/10 | 3 s | $0.016 |

- **Decision: Haiku at effort `low`** for Anthropic summaries. It beats
  qwen3:4b on every axis: no major error (qwen: 4), 3 more must-haves, 13
  more facts. `medium` was no better (2 must-haves fewer, same
  faithfulness), and on a single run the gap between the two is noise:
  `low` is enough. `add-anthropic-provider` assumed `medium`.
- **Cost**: 136,475 input + 3,745 output tokens for 10 videos at `low`
  (4,261 at `medium`), about $0.0016 per summary. Input dominates; effort
  barely changes it.
- One call per video: the 96,000-token input budget (131072 scaled as the
  app does) fits every case, where qwen3:4b map-reduces the long ones.
- Still opens with "El vídeo…"/"The video…" in 7 of 10 summaries at both
  efforts (`direct_start`): the code strip stays needed. One or two per run
  are 6 sentences.
- Caveats: one run per effort; `cross-entropy` and `extreme-places` used
  the subtitles yarrtube downloaded (same as the NAS copy, which a fresh
  `yt-dlp` download differs from), the rest a fresh download with the same
  text; case titles are now the original YouTube ones (see the v3 caveat).

## 2026-11-12 · budget the whole prompt, re-cut once on an overrun

A chess video (`Jobava London Guide…`) failed its map call "part 1 of 2" 4
times: 11,932 input tokens on 12,288, budget 9,000. Reproduced exactly with
local `qwen3:4b` from the real `.srt`:

- `\h` escapes (subtitle hard spaces), left at nearly every line end, cost
  ~2.2k tokens on that part (11,711 transcript tokens with them, 9,503
  without). In prod, 27 of 241 manual and 4 of 417 auto-generated transcripts
  had them.
- Clean chess English runs at 3.75 chars/token (4.3 assumed); Spanish parts
  measured 3.21–3.44 (3.4 assumed).
- The prompt around the transcript, uncounted until now, was ~220 tokens.

Now: escapes are cleaned, the budget counts each call's whole messages, and a
call that stops at the limit with input over the budget is cut again once at
its measured chars/token. The two summaries that failed on 2026-11-05 (7,997
and 9,470 input tokens) fit the budget: they hit the old 3,288-token output
cap, not this.

## 2026-11-05 · output limited only by the free context

Production capped output at what the input budget left (3288 of 12288) and
failed 17 calls on 10 videos in a day, some on inputs of ~2k tokens with ~10k
free. The cap sat inside normal use: the uncapped v3 run below has qwen3:4b
writing up to 3,551 tokens (`calls[].out` in its `summaries.jsonl`). A call now
uses all the context its prompt leaves. Inputs can still run over the budget
(11,131 seen), since the budget counts only the transcript, and leave little
room: retune it from the logged room left if those keep failing.

## 2026-11-03 · prompt v3 · qwen3:4b chosen

Run: [`2026-11-03_prompt-v3`](results/2026-11-03_prompt-v3/report-claude-sonnet.md),
compared with the v2 baseline below. v3 added one rule per v2 error type:
no invented conclusions, keep the speaker's hedging, don't present side
mentions as the subject, no numbers the transcript doesn't state, give the
reasoning behind a conclusion, one result per part, don't repeat the title.

| Prompt | Model | Must-haves | Facts | Major / minor unfaithful | Fully faithful | Avg time |
|---|---|---|---|---|---|---|
| v2 | qwen3:4b | 19/23 | 32/50 | 5 / 13 | 3/10 | 328 s |
| v2 | gemma3:4b | 14/23 | 22/50 | 2 / 10 | 1/10 | 92 s |
| v3 | **qwen3:4b** | **19/23** | **33/50** | **4 / 14** | 3/10 | 287 s |
| v3 | gemma3:4b | 12/23 | 20/50 | 10 / 12 | 2/10 | 87 s |

- **Decision: qwen3:4b + v3.** Most must-haves, fewest major errors. Speed
  doesn't matter for a background job.
- **gemma3:4b rejected**: v3 gave it 5× the major errors (made up an overall
  message, merged three chess games into one, reversed a presenter's advice).
  Extra rules seem to become things to write about rather than limits.
- **Stop tuning the prompt at 4B**: diminishing returns. Fix in code what
  code can fix.
- The chess stream fits in one call, with chapters marking each game, and
  both models still get its results wrong: a model limit, not chunking.
- Caveat: both runs gave the model the title stored when the case was
  created; for 4 videos it wasn't the original YouTube title (e.g. an
  English translation of a Spanish title). Cases now carry the original
  title, so the next run may score differently.

Next prompt version (v4): the reduce rules say "the part summaries is
auto-generated", a leftover of replacing "transcript"; fix it.

## 2026-11-02 · prompt v2 baseline · judge chosen

Run: [`2026-11-02_baseline_prompt-v2`](results/2026-11-02_baseline_prompt-v2/report-claude-sonnet.md).
First run of the eval: 10-video golden set, qwen3:4b and gemma3:4b. Scores
in the table above.

- **Judge: Claude Sonnet**, blind (it doesn't see which model wrote the
  summary). It agreed with the viewer on 4 of 6 verdicts. The local
  **qwen3:8b judge agreed on 0 of 4** and was too strict on paraphrases:
  kept in this run's folder for comparison, not used since.
- **Faithfulness check added**: fact coverage doesn't penalize wrong
  additions. It caught qwen inventing "374,142 bunkers" and a conclusion
  the video doesn't draw, and gemma presenting a criticized view as the
  video's own.
- The viewer prefers a summary that says less over one with a wrong detail.
- This run holds all the `human_verdicts` in the cases, so keep it until
  newer runs have some.

## 2026-10-30 · first benchmark, read by hand · qwen3:4b leading

No run folder: 9 transcripts read by eye, before the eval existed. MacBook
Air M2 8GB, Ollama 0.40.13, temperature 0.3.

Memory (decides which models and context sizes are possible):

| Model | num_ctx | RAM | Placement |
|---|---|---|---|
| qwen3:4b | 8k | 3.8GB | 100% GPU |
| qwen3:4b | 12k | 4.5GB | 100% GPU |
| qwen3:4b | 16k | 5.4GB | 16% CPU (slow) |
| gemma3:4b | 4k–16k | ~2.4GB | ~10% CPU always |

Speed per video:

| Model | Input tok/s | Output tok/s | Output tokens | Wall time |
|---|---|---|---|---|
| gemma3:4b | 155–187 | 14–17 | 100–180 | 23–60 s |
| qwen3:4b-instruct | 112–203 | 8–13 | 160–290 | 38–110 s |
| qwen3:4b (thinking) | 136–203 | 11–19 | 600–2,560 | 73–243 s |

- **12k tokens is qwen3:4b's ceiling** on 8GB: videos up to ~45–60 min fit
  one call; longer ones need map-reduce. Transcripts run ~1.3–1.6 tokens per
  word (30 min ≈ 8k tokens).
- **gemma3 summarizes the end of long inputs**: on a 3 h video split in
  ~8k-token chunks it described mostly each chunk's last minutes and lost a
  whole segment. qwen3 covered every segment.
- All models wrote in the video's language, including a Spanish video with
  an English title: use the transcript's language, not the title's.
- Small models ignore negative style rules ("do not start with 'The
  video'").
- qwen3-instruct inflated "almost 100 unions" to "more than 100"; qwen3 with
  thinking kept it. Thinking is slower but more accurate.
- Possible thermal throttling (fanless Air) after ~40 min of continuous
  load: matters for backlog runs. Not confirmed.

## Ollama gotchas

- Ollama's default context is small and it **truncates silently**: always set
  `num_ctx`, and compare `prompt_eval_count` with the expected input size.
- `qwen3:4b` **ignores `think: false`** and writes its reasoning into the
  answer: strip everything up to `</think>`.
- Thinking can loop for an hour: `shift: false` ends it at the context's end,
  and a timeout bounds the time. A `num_predict` below the free context fails
  normal calls too: keep it at the context length.
