# Categorization prompts

One file per version, holding only the system prompt as plain text. It wraps
the category prompt the user writes for each category. See
[evals/README.md](../../evals/README.md).

| Version | What changed | Eval run |
|---|---|---|
| v1 | Placeholder default from the AI category settings, on the `prepare-ai-categories` branch. Never evaluated. | none |
| v2 | Full rules: the description defines the category, a real part of the video is enough, no when unsure. **Shipped as the default, still under evaluation.** | [thinking off](../../evals/categories/results/2026-11-06_1248_prompt-v2_qwen3-4b_think-off/report.md), [thinking on](../../evals/categories/results/2026-11-06_1249_prompt-v2_qwen3-4b_think-on/report.md), 58 of 60 videos: nothing passes yet, see [decisions](../../evals/categories/decisions.md). On the tuned category prompts: [thinking on](../../evals/categories/results/2026-11-09_1958_prompt-v2_qwen3-4b_think-on/report.md), 12 wrong / 8 missed / 15 errors |
| v2-batch | v2's rules, asking about every category in one call per video: a video can fit several or none, each decided on its own. **Recommended over v2**: as good or better, a third of the output tokens. | [thinking on](../../evals/categories/results/2026-11-09_1903_prompt-v2-batch_qwen3-4b_think-on_batch/report.md), 8 wrong / 11 missed / 9 errors (one video hit the output cap), see [decisions](../../evals/categories/decisions.md) |
