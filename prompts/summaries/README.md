# Summary prompts

One file per version, holding only the prompt: `## <section>` headings, each
a message template. `{name}` placeholders are filled by the caller (see
[evals/README.md](../../evals/README.md) and the app's prompt guide on the AI
page). A video that fits the context uses `system` + `user`; a longer one is
cut into parts: `map system` + `map user` per part, then `reduce system` +
`reduce user` over the part summaries. `chapters`, `map chapters` and
`reduce part` are snippets inserted into those messages.

The app ships `v3.md` as the default prompt (embedded at build time).

| Version | What changed | Eval run |
|---|---|---|
| v2 | Baseline | `2026-11-02_baseline_prompt-v2` |
| v3 | v2 plus rules against invented conclusions, lost hedging, side mentions taken as the subject, invented numbers and title repetition. **In use.** | `2026-11-03_prompt-v3` |

Why v3 and the numbers behind it: [evals/summaries/decisions.md](../../evals/summaries/decisions.md).
