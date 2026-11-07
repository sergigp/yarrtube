# Categorization prompts

One file per version, holding only the system prompt as plain text. It wraps
the category prompt the user writes for each category. See
[evals/README.md](../../evals/README.md).

| Version | What changed | Eval run |
|---|---|---|
| v1 | Placeholder default from the AI category settings, on the `prepare-ai-categories` branch. Never evaluated. | none |
| v2 | Full rules: the description defines the category, a real part of the video is enough, no when unsure. **Under evaluation.** | [thinking off](../../evals/categories/results/2026-11-06_1248_prompt-v2_qwen3-4b_think-off/report.md), [thinking on](../../evals/categories/results/2026-11-06_1249_prompt-v2_qwen3-4b_think-on/report.md), 58 of 60 videos: nothing passes yet, see [decisions](../../evals/categories/decisions.md) |
