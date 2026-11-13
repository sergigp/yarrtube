# Categorization eval

Scores AI categorization: one model call per video + category, answering
whether the video belongs. What's in use and why: [decisions.md](decisions.md).
What evals are: [../README.md](../README.md).

## How it scores

You label each video in `labels.md`. Code compares the model's yes or no with
your labels, so no judge is needed:

- **Wrong**: the model said yes, you said no. The pass bar is at most 1 per
  category.
- **Missed**: the model said no, you said yes.
- **Precision**: of the videos it put in a category, how many belong there.
- **Recall**: of the videos that belong, how many it found.

Ignore accuracy: on a specific category, always answering no would score about
98%. A category also fails if any call gave no usable answer.

## Files

| File | What it is |
|---|---|
| `videos.json` | The 60 videos, with the summary and chapters the model sees |
| `labels.md` | Your labels. Only labeled videos are scored |
| `category-prompts.md` | The 9 category prompts, the `user` message and the JSON schema |
| `results/<run-id>/` | `run.json`, a copy of the category prompts, `verdicts.jsonl`, `report.md` |

The system prompt is `prompts/categorization/<version>.md`.

## Running it

Needs Ollama with the model pulled (`OLLAMA_URL` if it's on another machine),
or an Anthropic key for `--provider anthropic` (below), and `yt-dlp` for the
missing summaries (`--summarize` always uses Ollama).

```bash
python3 -I evals/categories/eval.py --check       # inputs valid? labels left?
python3 -I evals/categories/eval.py --summarize   # about 1 hour, commit videos.json after
python3 -I evals/categories/eval.py --model qwen3:4b --limit 10   # measure the time per call
python3 -I evals/categories/eval.py --model qwen3:4b              # the full 540 calls
python3 -I evals/categories/eval.py --model qwen3:4b --think      # again with thinking
```

- **Resume**: `--run <run-id>` continues a stopped run, and re-scores it after
  labels change, without calling the model again.
- **Change one category**: edit `category-prompts.md` and start a new run with
  `--reuse <run-id>`. Answers are copied when the settings and that category's
  prompt match, so only the changed category is asked again.
- **Other flags**: `--prompt`, `--num-ctx`, `--max-output` and `--temperature`
  for a new run.
- **Anthropic**: `--provider anthropic` runs the model on the Anthropic API
  instead of Ollama, with the key in `YARRTUBE_ANTHROPIC_API_KEY` (the run
  stops before its first call without it). `--effort low|medium|high`
  (default `medium`) is sent as `output_config.effort`; the model thinks
  adaptively, so `--think` and `--temperature` don't apply. Each call sends
  the schema as structured output and the output cap as `max_tokens`
  (`--num-ctx` only sizes that cap with `--batch`). A call stopped at
  `max_tokens`, a refusal or a 400 is an error row; a bad key, an unknown
  model, no credit or no connection stops the run, resumed with `--run`. A
  run's provider and effort are fixed when it starts, and its report ends
  with its token totals and an estimated cost.
- **Batch**: `--batch` asks as the app does: every category in one call per
  video, with prompt [`v2-batch`](../../prompts/categorization/v2-batch.md)
  and a schema keyed by category name. `num_ctx` is the app's Auto for the
  number of categories (16384 for 9) and each call's output cap is scaled to
  the categories it asks about (1,365 each, at least 3072); `--num-ctx` and
  `--max-output` override them. When a call about several categories is
  rejected or leaves some without a usable answer, each of those is asked
  alone, as the app's fallback does: those rows are marked `fallback` and keep
  the failed call's stats in `batch_call`, and the report's "Fallbacks" section
  counts them and the tokens spent on the calls that failed. Rows are scored
  like the others. To compare with one call per category, run both on the
  same machine and compare the report's output tokens per video: they don't
  depend on the hardware, seconds do.

```bash
python3 -I evals/categories/eval.py --model qwen3:4b --think           # one call per category
python3 -I evals/categories/eval.py --model qwen3:4b --think --batch   # one call per video
python3 -I evals/categories/eval.py --provider anthropic --model claude-haiku-5-5 --batch --effort low
```

`--limit 10` run first: thinking with an enforced JSON format hasn't been tried
yet, so check the report's errors before starting the full run.

Tune on these videos, then do a last check on 15-20 fresh ones: scores on the
videos you tuned against look better than they are.
