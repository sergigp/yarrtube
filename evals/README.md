# Evals

LLM evals for the AI features (see `doc/AI.md`). Today: video summaries.

```bash
python3 -I evals/summaries.py --check                                  # validate every case.json
python3 -I evals/summaries.py --model qwen3:4b --judge claude-sonnet   # new run, prompt in use
python3 -I evals/summaries.py --model qwen3:4b --prompt v2             # new run, another prompt
python3 -I evals/summaries.py --run <run-id> --judge claude-sonnet     # resume / ingest / re-report
```

Needs Ollama (`OLLAMA_URL`, default `http://localhost:11434`) and each case's
`subtitles.srt`, which is local only: the repo is public, so subtitles and
transcripts are gitignored. Fetch them with
`yt-dlp --write-auto-subs --sub-langs <lang> --convert-subs srt --skip-download <url>`.

## Layout

```
prompts/summaries/<version>.md     the prompt, one file per version (repo root, shared with the app)
evals/cases/<slug>/case.json       one golden-set video
evals/cases/<slug>/subtitles.srt   its captions (gitignored)
evals/results/<run-id>/            one run: <date>_prompt-<version>
```

## Prompt files

Markdown, one `## <name>` section per message template; text before the first
section is a description. `{name}` placeholders, filled in one pass:

| Section | Used for | Placeholders |
|---|---|---|
| `system`, `user` | videos that fit one call | `lang`; `title`, `minutes`, `chapters`, `transcript`, `lang` |
| `chapters` | the `{chapters}` block, only when the video has chapters | `chapter_list` |
| `map system`, `map user` | each part of a long video | `lang`; `title`, `minutes`, `part`, `parts`, `span`, `chapters`, `transcript`, `lang` |
| `map chapters` | the chapters inside one part | `chapter_list` |
| `reduce system`, `reduce user` | final summary of a long video | `lang`; `title`, `minutes`, `chapters`, `part_summaries`, `lang` |
| `reduce part` | one entry of `{part_summaries}` | `part`, `span`, `summary` |

An evaluated version is never edited; a change is a new file. The version in
use is `PROMPT_VERSION` in `summaries.py` and is recorded in `doc/AI.md`.

## case.json

```jsonc
{
  "youtube_id": "my5hHTVZ_NI",
  "url": "https://www.youtube.com/watch?v=my5hHTVZ_NI",   // must match youtube_id
  "title": "De 20 minutos a 1 segundo: …",   // original YouTube title (goes in the prompt)
  "yarrtube_title": "browser tabs",          // optional: title yarrtube had, when different
  "channel": "CodelyTV - Redescubre la programación",
  "published": "2026-10-06",                 // YYYY-MM-DD
  "lang": "es",                              // spoken language; the summary's language
  "duration_s": 472,
  "chapters": [{"title": "…", "start_seconds": 0, "end_seconds": 120}],   // [] if none
  "description": "…",                        // context for reviewers, not sent to the model
  "summary": {
    "facts": [{
      "id": "built-browser-tabs-cli",        // unique, kebab-case
      "fact": "…",                           // one main claim, stated first
      "critical": true,                      // a good 3–5 sentence summary must say it
      "trap": "…",                           // optional: the usual wrong version
      "note": "…",                           // optional: what counts as stated, for the judge
      "evidence": "…"                        // optional: timestamp + quote from the transcript
    }],
    "review": {                              // optional until the facts are reviewed
      "reviewed_on": "2026-11-02",
      "by": "viewer",                        // or who/what checked it, e.g. "claude-sonnet against the transcript (not a viewer)"
      "note": "…",
      "human_verdicts": {                    // optional: viewer's verdicts, to measure judge agreement
        "run": "2026-11-02_baseline_prompt-v2",
        "qwen3:4b": {"built-browser-tabs-cli": "stated"}   // stated | contradicted | missing
      }
    }
  }
}
```

`summaries.py` validates every case on load and stops on the first bad one.

## A run

`evals/results/<run-id>/`:

- `run.json`: models, prompt version, `num_ctx`. A resumed run keeps its prompt.
- `summaries.jsonl`: one row per case and model, with per-call timings and
  code checks (language, 3–5 sentences, no leftovers, direct start).
- `judgments-<judge>.jsonl`: per fact, stated / contradicted / missing.
- `faithfulness-<judge>.jsonl`: claims the transcript doesn't support, major or minor.
- `report-<judge>.md`: the tables and every failure.

With `--judge local` the judge is qwen3:8b via Ollama (unreliable, kept for
comparison). With an external judge name the script writes blind inputs
(named by a hash, so the judge can't see the model) to `judge-external/in/`
and `faithfulness-external/in/`, each with an `INSTRUCTIONS.md`, and stops.
One Claude subagent per input writes `out/<key>.json`; the next run ingests
them.
