# Summaries eval

Scores the video summary feature: a 3–5 sentence summary per video, written
from its transcript in the video's language. What's in use and why:
[decisions.md](decisions.md). What evals are and the shared ideas (golden
set, judges, noise): [../README.md](../README.md).

## How a summary is scored

Each video in the golden set (`cases/`) comes with a few **facts**: short
statements a correct summary should contain, e.g. "the streamer plays three
games: two wins and a draw". Facts that any good 3–5 sentence summary must
include are marked **critical** (the "must-haves"). A fact can also carry a
**trap**: the usual way models get it wrong.

Every summary gets three kinds of checks:

1. **Code checks** (instant, free): right language, 3–5 sentences, no
   leftovers like URLs or "subscribe", doesn't open with "The video…".
2. **Fact coverage**: for each fact, a judge says whether the summary states
   it, contradicts it, or leaves it out. High coverage means the summary
   says the important things.
3. **Faithfulness**: a judge reads the *whole transcript* and lists every
   claim in the summary the video doesn't support: invented numbers, wrong
   results, a side mention presented as the main topic. Each one is graded
   **major** (changes what a viewer would take away) or **minor**.

Coverage alone misleads: a summary can cover every fact and still invent a
wrong detail. **A summary with less coverage and no wrong details beats one
with more coverage and a major error.** When comparing runs, look at
must-haves and major errors first.

The judge is Claude Sonnet, run as Claude Code subagents, blind to which
model wrote each summary. `--judge local` uses qwen3:8b via Ollama instead;
it's unreliable and kept only for comparison.

## Running it

### Before the first run

- Ollama running on the laptop, with the models pulled
  (`ollama pull qwen3:4b`). If it's not on this machine, set
  `OLLAMA_URL=http://<host>:11434`.
- Each case's subtitles. **The repo is public, so subtitles and transcripts
  are never committed** (`evals/.gitignore` blocks them). On a new machine,
  download them again:

  ```bash
  cd evals/summaries/cases/<slug>
  yt-dlp --write-auto-subs --sub-langs <lang> --convert-subs srt --skip-download \
    -o subtitles "<url from case.json>"
  mv subtitles.<lang>.srt subtitles.srt
  ```

- Check that the cases are well formed:
  `python3 -I evals/summaries/eval.py --check`.

### 1. Generate the summaries

```bash
python3 -I evals/summaries/eval.py --model qwen3:4b --judge claude-sonnet
```

- Add `--model` again to compare several models in one run, e.g.
  `--model qwen3:4b --model gemma3:4b`.
- `--prompt v2` runs another prompt version (default: the one in use).
- `--case <slug>` limits the run to some videos, for a quick try.
- Slow: about 5 min per video for qwen3:4b, so ~50 min per model. Run it in
  the background. If it stops, rerun the same command with
  `--run <run-id>` and it picks up where it stopped; every summary is cached.

The run's folder is `evals/summaries/results/<date>_prompt-<version>/`. When
the summaries are done, the script writes the judge inputs and stops, saying
what is waiting for the external judge.

### 2. Have Sonnet judge them

In Claude Code, from the repo, ask:

> Judge the pending eval inputs in `evals/summaries/results/<run-id>/`: one
> Sonnet subagent per input key. Each one follows
> `judge-external/INSTRUCTIONS.md` on `judge-external/in/<key>.md` and
> `faithfulness-external/INSTRUCTIONS.md` on
> `faithfulness-external/in/<key>.md`, and writes both `out/<key>.json`.
> Then rerun the script with `--run <run-id> --judge claude-sonnet`.

A run of 10 videos for 2 models is 20 subagents, done in under a minute.

### 3. Read the report

```bash
python3 -I evals/summaries/eval.py --run <run-id> --judge claude-sonnet
```

This ingests the answers and writes `report-claude-sonnet.md` in the run
folder: a table per model, then every failed must-have, every unfaithful
claim with the transcript timestamp, and failed code checks. Compare it with
the report of the run in use (linked from [decisions.md](decisions.md)).

### 4. Decide, and record it

If the change wins: add an entry at the top of [decisions.md](decisions.md)
(what changed, the scores, the decision), update its "In use" section and
`PROMPT_VERSION` in `eval.py`, then commit the run folder together with the
change. If it loses, still log it when it taught something; otherwise delete
the run.

## Common tasks

**Try a new prompt.** Copy the current file to the next version
(`prompts/summaries/v4.md`) and edit the copy. Never edit a version that has
already been evaluated. Run with `--prompt v4`.

**Try a new model.** `ollama pull <model>`, then run with `--model <model>`
and the current model, so both are scored in the same run. Mind the 8GB
RAM: see the memory table in [decisions.md](decisions.md).

**Add a video to the golden set.** Best with a video you've watched:
1. Create `cases/<slug>/case.json` (format below) and download its
   subtitles.
2. Write 3–6 facts: the topic, the main points, how it ends. One claim per
   fact, the main claim first. Mark the must-haves `critical` and add a
   `trap` when there's an obvious wrong version.
3. Get the facts checked, by you (`by: "viewer"`) or by Claude against the
   transcript, and fill in `review`.
4. `--check`, then run the eval.

**Record your own verdict.** When you read a summary and disagree with the
judge, add it to `review.human_verdicts` for that run. It's how we know
whether to trust the judge.

## Reference

### Layout

```
prompts/summaries/<version>.md            the prompt, one file per version (shared with the app)
evals/summaries/eval.py                   the eval script
evals/summaries/decisions.md              what's in use, and why
evals/summaries/cases/<slug>/case.json    one golden-set video
evals/summaries/cases/<slug>/subtitles.srt its captions (gitignored)
evals/summaries/results/<run-id>/         one run: <date>_prompt-<version>
```

### Prompt files

Markdown, one `## <name>` section per message template; text before the first
section is a description. `{name}` placeholders, filled in one pass.

A video that fits in one call (up to ~45–60 min) uses `system` + `user`.
Longer ones are split into parts: each part is summarized (`map`), then the
part summaries are combined into the final summary (`reduce`).

| Section | Used for | Placeholders |
|---|---|---|
| `system`, `user` | videos that fit one call | `lang`; `title`, `minutes`, `chapters`, `transcript`, `lang` |
| `chapters` | the `{chapters}` block, only when the video has chapters | `chapter_list` |
| `map system`, `map user` | each part of a long video | `lang`; `title`, `minutes`, `part`, `parts`, `span`, `chapters`, `transcript`, `lang` |
| `map chapters` | the chapters inside one part | `chapter_list` |
| `reduce system`, `reduce user` | final summary of a long video | `lang`; `title`, `minutes`, `chapters`, `part_summaries`, `lang` |
| `reduce part` | one entry of `{part_summaries}` | `part`, `span`, `summary` |

### case.json

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
      "human_verdicts": {                    // optional: your verdicts, to measure judge agreement
        "run": "2026-11-02_baseline_prompt-v2",
        "qwen3:4b": {"built-browser-tabs-cli": "stated"}   // stated | contradicted | missing
      }
    }
  }
}
```

`eval.py` validates every case on load and stops on the first bad one.

### A run's files

- `run.json`: models, prompt version, `num_ctx`. A resumed run keeps its prompt.
- `summaries.jsonl`: one row per case and model, with per-call timings and
  code checks.
- `judgments-<judge>.jsonl`: per fact, stated / contradicted / missing.
- `faithfulness-<judge>.jsonl`: claims the transcript doesn't support, major or minor.
- `report-<judge>.md`: the tables and every failure.
- `judge-external/`, `faithfulness-external/`: scratch for the blind judges:
  inputs (`in/`), instructions, answers (`out/`) and transcript copies. Not
  committed: the answers are copied into the `.jsonl` files when ingested,
  and the inputs can be rebuilt from `summaries.jsonl`.
