## Why

The tasks view's tabs don't match how the tasks are actually watched. Syncs mixes per-channel and per-playlist reconciles with system jobs, Cleanup and All add noise, and a sync can't be kicked off from the view that shows it. The tab bar itself barely reads as clickable: a near-white active pill on a light-grey track, faded labels, and count badges the same grey as the track.

## What Changes

- **BREAKING (UI)**: the tasks view tabs become **Active**, **Downloads**, **Syncs**, **Other**. The **Cleanup** and **All** tabs are removed.
- **Syncs** lists only playlist and channel reconciliation tasks.
- **Other** is the catch-all for every task not in Downloads or Syncs: Plex collection sync, yt-dlp self-update, file/container deletions, and any unknown type.
- Each task row's icon is picked by task type rather than by tab, so rows in **Other** stay distinguishable.
- Each row in **Syncs** gets a **Run now** action that triggers the existing on-demand reconcile endpoint for that playlist or channel. The endpoint is unchanged: it runs one pass immediately and leaves the scheduled reconcile as it was.
- The task listing endpoint includes the playlist id or channel handle on reconcile tasks, so the UI can address the reconcile endpoint.
- The tab bar is redesigned as underline tabs: green active indicator, bold active label, hover background, an icon per tab, count pills (green on the active tab), a live beacon on the Active count while tasks are running, at least 40px tall tabs, a visible focus ring, and horizontal scrolling on narrow screens.

## Capabilities

### New Capabilities

### Modified Capabilities
- `web-ui`: the tasks view tab set and membership change, Syncs rows gain Run now, and the tab bar gains visual requirements.
- `task-listing`: reconcile tasks include the identifier of the playlist or channel they concern.

## Impact

- Frontend: `web/src/lib/tasks.ts` (families, tabs, icons), `web/src/components/TasksView.tsx`, `web/src/components/ui/tabs.tsx` (`line` variant restyle), `web/src/api/queries.ts`, and the colocated tests.
- Backend: `src/domain/services/task_view_searcher.rs` adds ids to the reconcile task view, plus tests in `src/application/http/tasks/mod.rs`.
- No change to the reconcile endpoints, task scheduling, or the database.
