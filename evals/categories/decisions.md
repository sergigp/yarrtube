# Categorization: decision log

What video categorization uses and why. Newest decision first. How the eval
works: [README.md](README.md).

## In use

Nothing yet: prompt [`v2`](../../prompts/categorization/v2.md) and the
[category prompts](category-prompts.md) are under evaluation.

## 2026-11-06 · design, before any run

- **One call per video + category**, not all categories in one call: a video
  can fit none or several, editing one category re-runs only that category,
  and rules can't bleed between categories. Unmeasured: the eval times it.
- **Summaries are required**, and chapters are sent with them. Summaries can
  get details wrong but rarely the subject. Known risk: a stream with many
  openings may lose the one the category needs.
- **A real part of the video is enough**: one Sicilian game in a stream counts,
  a passing mention doesn't. The risk is broad categories catching too much.
- **The pass bar is at most 1 wrong video per category**, set before any run.
  Report counts, not percentages: with 5 matches one miss is already 20%.
- **Golden set**: 30 random videos with summaries in prod, 20 hand-picked for
  Jobava, Sicilian and near-misses, plus the 10 summary cases. Prod summaries
  exist only for recent channel videos, so the random part reflects those.
- **Starting settings**: temperature 0, `num_ctx` 4096, thinking compared on
  and off.
