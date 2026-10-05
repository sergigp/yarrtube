## Files

- `src/domain/services/task_view_searcher.rs` — reconcile task views add `playlist_id` / `channel_id` from the decoded payload, set before the name lookup so they survive a missing playlist/channel.
- `src/application/http/tasks/mod.rs` — handler tests: new id tests; existing reconcile tests' expected payloads gain the id.
- `web/src/lib/tasks.ts` — families `downloads | syncs | other` (unlisted → `other`), tabs `active | downloads | syncs | other`, `taskKind` for row icons, `syncTarget` for Run now.
- `web/src/lib/tasks.test.ts` — unit tests for the above.
- `web/src/api/queries.ts` — `useInvalidateTasks` so Run now refetches the task list.
- `web/src/components/ui/tabs.tsx` — restyle the `line` variant into the underline tab bar (primary indicator, hover bg, ≥40px, equal-width tabs). The only tabs consumer is TasksView.
- `web/src/components/TasksView.tsx` — view centred at `max-w-3xl`, new tabs with icon + label (hidden below `sm`) + count pill + Active live beacon, per-type row icons, private `RunNowButton` on Syncs rows.
- `web/src/components/TasksView.test.tsx` — component tests for tabs and Run now.

No change to `web/src/api/types.ts`: `Task.payload` is already `Record<string, string>`.

## Types & Signatures

```rust
// task_view_searcher.rs — payload keys added (no new types)
// reconcile_playlist: { "playlist_id": <id>, "playlist_name"?: <name> }
// reconcile_channel:  { "channel_id": <handle>, "channel_name"?: <name> }
fn resolve_reconcile_playlist(&self, payload: &str) -> anyhow::Result<HashMap<String, String>>;
fn resolve_reconcile_channel(&self, payload: &str) -> anyhow::Result<HashMap<String, String>>;
```

```ts
// lib/tasks.ts
export type TaskFamily = 'downloads' | 'syncs' | 'other'
export type TaskTab = 'active' | 'downloads' | 'syncs' | 'other'
export const TASK_TABS: readonly TaskTab[] = ['active', 'downloads', 'syncs', 'other']
export function taskFamily(task: Task): TaskFamily            // unlisted type -> 'other'
export function tasksForTab(tasks: Task[], tab: TaskTab): Task[]
export function tabCounts(tasks: Task[]): Record<TaskTab, number>

export type TaskKind = 'download' | 'sync' | 'delete' | 'maintenance'
export function taskKind(task: Task): TaskKind                // drives the row icon

export type SyncTarget = { kind: 'playlist' | 'channel'; id: string }
export function syncTarget(task: Task): SyncTarget | null     // null unless pending reconcile_playlist/channel with an id

// api/queries.ts
export function useInvalidateTasks(): () => Promise<unknown>

// TasksView.tsx (private)
const TAB_ICON: Record<TaskTab, LucideIcon>                   // Activity, Download, RotateCw, Wrench
const KIND_ICON: Record<TaskKind, LucideIcon>                 // Download, RotateCw, Trash2, Wrench
function RunNowButton({ target, name }: { target: SyncTarget; name: string }): JSX.Element
```

## Call Stack

Listing (backend):
```
GET /api/tasks
  -> tasks::list_tasks
    -> TaskViewSearcher::search
      -> resolve_reconcile_playlist(payload)
           Task::decode_reconcile_playlist_payload(payload) -> id
           view.insert("playlist_id", id)
           playlist_repository.find(&PlaylistId) -> Some => view.insert("playlist_name", name)
      -> resolve_reconcile_channel(payload)   (same shape, "channel_id"/"channel_name")
```

Tabs (frontend):
```
TasksView
  -> useTasks()                       -> tasks
  -> tabCounts(tasks), tasksForTab(tasks, tab)
  -> Tabs > TabsList variant="line"
       TabsTrigger: TAB_ICON[tab] + label + count pill (+ Beacon on Active when counts.active > 0)
  -> row: KIND_ICON[taskKind(task)], describeTask(task)
       tab === 'syncs' && syncTarget(task) -> <RunNowButton target name=describeTask(task)>
```

Run now:
```
RunNowButton onClick
  -> setRunning(true), setError(null)
  -> target.kind === 'playlist' ? reconcilePlaylist(target.id) : reconcileChannel(target.id)
       -> POST /api/playlists/{id}/reconcile | /api/channels/{handle}/reconcile   (204, blocks for the pass)
  -> ok:    invalidateTasks() + invalidateLibrary()
  -> error: setError(message)  (shown inline on the row, destructive text)
  -> finally setRunning(false)        (button disabled + RotateCw animate-spin while running)
```

## Test Plan

Behaviour tests — HTTP handler (`src/application/http/tasks/mod.rs`):
1. `it_should_include_playlist_id_in_reconcile_playlist_tasks` — tracked playlist: payload is `{playlist_id: "PL1", playlist_name: "My Playlist"}`.
2. `it_should_keep_playlist_id_if_playlist_is_gone` — replaces `it_should_omit_details_if_playlist_is_gone`: payload is `{playlist_id: "PL1"}`.
3. `it_should_include_channel_id_in_reconcile_channel_tasks` — tracked channel: payload is `{channel_id, channel_name}`.
4. `it_should_keep_channel_id_if_channel_is_gone` — replaces `it_should_omit_details_if_channel_is_gone`: payload is `{channel_id}`.

(Existing `it_should_include_{playlist,channel}_name_in_reconcile_*_tasks` are subsumed by 1 and 3 and removed.)

Frontend unit tests (`web/src/lib/tasks.test.ts`, replacing the family/tab tests):
5. `maps each task type to downloads, syncs or other, and unknown types to other`
6. `lists only playlist and channel reconciles in the syncs tab`
7. `collects Plex sync, yt-dlp update, deletions and unknown types in the other tab`
8. `returns counts where downloads, syncs and other add up to every task`
9. `maps each task type to its icon kind`
10. `returns a sync target for pending playlist and channel reconciles only` — null for running, Plex, missing id.

Frontend component tests (`web/src/components/TasksView.test.tsx`):
11. `shows Active, Downloads, Syncs and Other tabs with counts, and no Cleanup or All tab`
12. `shows a live indicator on the Active tab only while a task is running`
13. `runs a playlist sync now from the Syncs tab and refetches the tasks` — routes `POST /api/playlists/PL1/reconcile`, asserts tasks refetched.
14. `runs a channel sync now from the Syncs tab`
15. `disables Run now while the sync is in progress` — `pendingForever()` on the reconcile route.
16. `shows the failure on the row and lets Run now be retried` — `{ status: 500, error }`.
17. `offers no Run now on a running sync or outside the Syncs tab`

Existing tests asserting Cleanup/All tabs or the old families are removed or rewritten by 5–8 and 11. No infrastructure adapter changes.
