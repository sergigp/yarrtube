# Evals

Tests for the AI features, one folder per feature:

| Feature | Folder | Status |
|---|---|---|
| Video summaries | [`summaries/`](summaries/README.md) | in use: see its [decision log](summaries/decisions.md) |
| Auto-categorization | [`categories/`](categories/README.md) | prompts under evaluation: see its [decision log](categories/decisions.md) |

This page explains what evals are and the ideas every feature shares. How to
run one and what it scores is in the feature's own README.

## Why this exists

Normal tests check code that always gives the same answer. An LLM doesn't:
change one word in the prompt, or swap the model, and the output changes,
sometimes for the better on one video and worse on another. Reading a few
outputs by eye isn't enough. They read well even when a detail is wrong,
and you can't remember what last week's version said.

An **eval** is a fixed set of real inputs with known good answers, scored
the same way every time. It turns "I think the new prompt is better" into
numbers you can compare:

```
change something (prompt, model, settings)
  → run it on the same fixed set of videos
  → score the output the same way
  → compare with the previous run
  → keep the change only if the numbers say it's better
```

**Run a feature's eval whenever you want to change its model, its prompt or
its generation settings**, and before shipping that change.

## Shared ideas

- **Golden set**: real videos (`<feature>/cases/`) with the known good
  answer written down and checked by a person. It's the expensive part to
  build; keep it.
- **Prompts are versioned files** in `prompts/<feature>/<version>.md` at the
  repo root, read by both the eval and the app. An evaluated version is never
  edited: a change is a new version, so old results always match their
  prompt.
- **Runs** (`<feature>/results/<date>_prompt-<version>/`) store the outputs,
  the scores and a readable report. Keep the outputs, not just the scores:
  when the golden set or the judge changes, old outputs can be re-scored in
  a minute instead of regenerated in hours.
- **Decision log** (`<feature>/decisions.md`): what's in use, and for each
  decision the runs and numbers behind it. Keep a run only while the log
  cites it; git history keeps the rest.

### LLM as judge

When there's no single right answer (a summary can be worded many ways),
another LLM scores the output against the golden set. Lessons so far:

- **Use a strong judge.** A local 8B model agreed with a person 0 times out
  of 4; Claude Sonnet, run as Claude Code subagents, 4 out of 6.
- **Keep it blind.** The judge gets the output in a file named by a hash,
  so it can't tell which model wrote it.
- **Check the judge against a person.** Record your own verdicts on a few
  outputs (`human_verdicts`); the report shows how often the judge agrees.
- **One claim per expected fact**, main claim first. Bundled facts confuse
  judges.

When there is a right answer (does this video belong in "Chess"? yes or no),
no judge is needed: compare with the labels and count. That's the plan for
categorization, measuring **precision** (of the videos it put in Chess, how
many really are) and **recall** (of the real Chess videos, how many it
found).

### Reading the numbers

- **Small differences are noise.** There's one output per video and model,
  and LLMs vary from run to run. Don't trust a gap of 1–2 points; trust
  bigger gaps, or the same gap across several runs.
- **Overfitting.** Adding a prompt rule for each mistake seen on a small
  golden set can make it look better without helping on other videos. Keep
  rules generic and add new cases now and then.

## Adding a feature

Create `evals/<feature>/` with `eval.py`, `cases/`, `results/`, a
`README.md` (what it scores, how to run it, file formats) and a
`decisions.md`, plus `prompts/<feature>/v1.md`. Add it to the table above.
Subtitles and judge scratch folders are already gitignored for any feature
(`evals/.gitignore`).
