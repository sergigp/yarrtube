# Proposal

## Why

Videos get AI summaries, but nothing displays them: the detail pane's description slot is empty since the YouTube description was removed. Showing a summary needs a strict rule, because turning AI off anywhere means "no AI here", even for summaries already generated.

## What Changes

- The video detail pane shows a video's summary, in full, under the meta line, labelled "AI summary". This covers playlist, channel and category views.
- The playlist, channel and category video listings return a `summary`, absent when the video has none or it is hidden.
- A stored summary is hidden unless AI is on, summaries are on and the video's own source has `summaries_enabled`. Hiding is decided by the backend, so the API never sends a hidden summary.
- Hidden summaries stay stored and come back, without regenerating, when the settings are turned back on.
- Each source's copy of a video is judged by that source's toggle. A category view lists the channel copy when there is one, so the channel's toggle decides there.
- Saving preferences refetches video lists, so a summary doesn't linger on screen after it's turned off.
- The "Summarize videos" hint becomes "Generate and show an AI summary of each video.", since the toggle now also controls display.
- Unchanged: nothing is shown for videos still waiting for a summary or given up on, and the NFO `<plot>` keeps the YouTube description.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `video-summaries`: when a stored summary may be shown.
- `video-listing`: listed videos include their summary.
- `web-ui`: the detail pane shows the summary; the checkbox hint changes; preferences refetch video lists.

## Impact

- **Domain**: `VideoSearcher` gains `VideoSummaryRepository` and decides visibility; `VideoView` and `CategoryVideoView` carry the summary.
- **API**: `VideoResponse` and `CategoryVideoResponse` gain `summary`, additive.
- **Web**: `VideoDetail`, `SummariesField`, `useSavePreferences`, `web/src/api/types.ts`, the `aVideo` builder.
- **Docs**: `doc/AI.md`, `doc/ARCHITECTURE.md`.
- **Not affected**: database, summary generation, NFO files.
