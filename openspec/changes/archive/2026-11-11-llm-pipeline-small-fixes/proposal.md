# Proposal

## Why

Three small gaps in the summaries → categorization pipeline waste hours or
minutes on the NAS: a fresh summary waits for the source's next reconcile pass
(possibly hours) before its categorization is scheduled; changing the
categorization thinking, context length or temperature (exactly what one tunes
after an output-limit failure) leaves failed categorizations failed; and a
categorization whose pending categories vanished before it ran still makes a
minutes-long LLM call about zero categories.

## What Changes

- **Sync after a summary is stored.** `VideoSummarizer` publishes a new
  `VideoSummaryGenerated` domain event once the summary is stored and the
  video marked Done; a new `SyncVideoOnVideoSummaryGenerated` subscriber
  schedules a `SyncVideo` now, mirroring `VideoSubtitlesFetched` /
  `VideoDownloaded`. The sync then decides whether categorization is wanted.
  No event when the generated summary is dropped (video no longer wants one).
  Publishing is best effort, like `VideoSubtitlesFetched`: a lost event only
  delays categorization to the next reconcile pass.
  - This reverses a line of the 2026-11-07 `categorize-videos-on-reconcile`
    design ("By design nothing syncs a video when its summary is generated"),
    mirrored in `video-categorization` ("Generating a summary SHALL NOT itself
    sync the video"). No reason was recorded for it; the event path the other
    steps use didn't exist for summaries then. Overridden here on request.
- **Every categorization call setting retries failed categorizations.**
  `Preferences::changes_categorization` compares every categorization setting
  except `enabled` (model, context length, temperature, thinking, effective
  prompt) plus the Ollama URL, mirroring `changes_summaries`. Context length
  is compared as saved (Auto vs a number), so switching Auto to a number
  counts as a change even if it resolves to the same length.
  - Checked: the narrow comparison (URL, model, prompt) came in with
    `categorize-videos-on-reconcile` (#166) with no recorded rationale,
    although context length, temperature and thinking were already sent on
    every call. Treated as an omission.
  - Side effect, intended: `VideoCategorizer::unchanged_since` uses the same
    method, so a change to thinking, context length or temperature during a
    call now also drops that call's verdicts and keeps a last attempt from
    writing `failed` verdicts, as a model change already does.
- **No LLM call when nothing is pending.** `VideoCategorizer::prepare`
  returns a new skip when `pending` is empty, before the server check. The
  video's categorization is marked Done (`mark_categorized`, failed attempts
  cleared), because Done means "the last run left nothing rejected" and
  nothing is pending: the categorization has nothing left to do until a
  category change makes it Pending again (`CategorizationResetter`). Leaving
  it Pending or Retrying would show a categorization that never resolves,
  since the sync never schedules one with nothing pending.
- **A last attempt overtaken by a settings change goes back to Pending.**
  When the settings (URL, model, context length, temperature, thinking,
  prompt) or a category's AI prompt change while a video's last attempt runs,
  that attempt drops its `failed` verdicts but still recorded Failed, and
  nothing ever reset it. After giving up, a categorization that still has a
  category pending goes back to Pending (through `CategorizationResetter`), so
  the next sync asks again with the new settings. The race predates this
  change (model, URL, prompt), but widening `changes_categorization` makes it
  much easier to hit, so it is fixed here.

Out of scope: retrying output-limit stops, the summarizer redoing every part
after a failure, keep_alive / model loading, attempts and backoff.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `video-categorization`: "When A Video Is Categorized" (a stored summary
  syncs the video now), "Categorization Run" (nothing pending → Done with no
  call), "Failed Verdicts Reset" (every call setting; a last attempt
  overtaken by a change goes back to Pending).
- `video-summaries`: "Summary Stored" (a stored summary syncs the video).
- `preferences`: "Categorization Settings" (changing any call setting resets
  failed categorizations).

## Impact

- Rust: new event in `src/domain/video/events.rs` + `DomainEvent` variant,
  new subscriber registered in `src/application/subscribers/mod.rs`,
  `VideoSummarizer` gains an `EventPublisher` (wired in `src/serve.rs` and
  test helpers), `Preferences::changes_categorization`, `VideoCategorizer`
  skip, `VideoCategorizer` gains a `CategorizationResetter`.
- New outbox event type `video_summary_generated` (payload `{ video_id }`).
  No HTTP API, DB schema or web UI change.
- One more `SyncVideo` per summarized video; syncs are cheap and deduped.
