# Proposal

## Why

Sources can opt in to AI summaries and the settings exist, but nothing generates a summary: the toggles have no effect and the stored prompt and transcripts go unused.

## What Changes

- The video sync schedules a summary for a downloaded video with fetched subtitles when AI and summaries are on and its channel or playlist has summaries enabled. This backfills every eligible stored video of the source.
- A new background task fills the effective prompt (the saved one, else the shipped one) with the video's title, duration, language, chapters and transcript and sends it to Ollama with the saved model, context length, temperature and thinking settings.
- Transcripts over the context budget are summarized by map-reduce, cut at chapter boundaries when the video has chapters, as measured in `evals/summaries`.
- A new summary lane runs one generation at a time, so a backlog doesn't hold other lanes back. Like any running task, a summary still holds back an exclusive task (`update_ytdlp`) and whatever waits behind it until it finishes.
- Summaries are stored, one per video, and never regenerated. A failed generation goes through the task queue's retries like any task. When they run out, a bad result gives the video up as Unavailable, while an Ollama fault leaves it for a later sync. Ollama being down or the model missing skips the run without failing it.
- Ollama is told never to truncate a prompt, so an oversized one is refused instead of silently summarized in part.
- Changing the summary settings or the Ollama URL puts every Unavailable video back in line.
- A finished subtitle fetch triggers a video sync, so a new video's summary doesn't wait for the next reconcile pass.
- Summary tasks show in the task list with their video's title.
- The whole process is logged at info level: full prompts, answers, per-call and total durations, and the video's length.
- Nothing displays a stored summary yet: the API, web UI video views and NFO `<plot>` are left for a follow-up.

## Capabilities

### New Capabilities

- `video-summaries`: generating, storing and retrying a video's AI summary from its transcript.

### Modified Capabilities

- `video-sync`: schedules the summary step; a subtitle fetch triggers a sync.
- `task-scheduling`: new summary lane; summary tasks take part in per-video exclusion.
- `task-listing`: summary tasks carry video context.
- `source-summaries`: the setting now has an effect.

## Impact

- **Database**: new `video_summaries` table and a `summary_state` column on `videos` (migration `0023`).
- **Domain**: new `video_summary` aggregate and `VideoSummarizer` service; `VideoOwner` and `VideoSyncPlan` learn whether summaries are wanted; `Task`, `TaskLane`, `VideoSyncStep` grow a summary variant.
- **Infrastructure**: `LlmServerRepository` gains a chat call (`POST /api/chat`, native API, because the OpenAI-compatible one can't set `num_ctx`).
- **Web**: task description and family for `summarize_video`.
- **Docs**: `doc/ARCHITECTURE.md`, `doc/AI.md`.
