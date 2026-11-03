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
- Thinking can loop for an hour: cap it with `num_predict` and a timeout.
