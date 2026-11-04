## Why

Summaries can queue hundreds of tasks, and they all land in Other, burying maintenance tasks. AI work deserves its own tab, shown only to those using AI.

## What Changes

- The tasks view gains an **AI** tab, between Syncs and Other, listing AI tasks. Today that's video summaries.
- The tab shows only while AI is enabled in settings, and not while preferences are loading or failed to load.
- While AI is off, a leftover summarize task is listed under Other, so every task stays in a tab.
- AI task rows get their own icon.
- Unchanged: the backend, and how a summarize task is described.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: tasks view tabs.

## Impact

- **Web**: `web/src/lib/tasks.ts`, `web/src/components/TasksView.tsx`, `web/src/api/queries.ts`.
- **Not affected**: backend, API, database, docs.
