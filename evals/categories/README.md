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

Needs Ollama with the model pulled (`OLLAMA_URL` if it's on another machine)
and `yt-dlp` for the missing summaries.

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
- **Other flags**: `--prompt`, `--num-ctx` and `--temperature` for a new run.

`--limit 10` run first: thinking with an enforced JSON format hasn't been tried
yet, so check the report's errors before starting the full run.

Tune on these videos, then do a last check on 15-20 fresh ones: scores on the
videos you tuned against look better than they are.
