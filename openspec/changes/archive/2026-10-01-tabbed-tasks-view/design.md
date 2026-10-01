# Design

Frontend-only change (TypeScript/React/Vitest), so the sections below are
adapted to `web/`: the "Test Plan" groups are component-behaviour tests
(`TasksView.test.tsx`) and pure-logic unit tests (`lib/tasks.test.ts`) instead
of the Rust behaviour/infrastructure split. See proposal.md for the why.

## Files

- `web/src/lib/tasks.ts` — add tab/family grouping, counts, and search
  predicate; extend `describeTask` so every type and the unknown fallback are
  human-readable. Pure logic lives here per the frontend conventions.
- `web/src/lib/tasks.test.ts` — unit tests for the new pure functions.
- `web/src/components/TasksView.tsx` — render the tabs, per-tab counts, the
  search field, and the improved rows (family icon + last error).
- `web/src/components/TasksView.test.tsx` — component-behaviour tests.
- Uses existing `web/src/components/ui/{tabs,tooltip,input}.tsx` and `lucide`
  icons; no new files or dependencies.

## Types & Signatures

```ts
// lib/tasks.ts
export type TaskFamily = 'downloads' | 'syncs' | 'cleanup' | 'maintenance'
export type TaskTab = 'active' | 'downloads' | 'syncs' | 'cleanup' | 'all'

export const TASK_TABS: readonly TaskTab[] // ['active','downloads','syncs','cleanup','all']
export const TASK_SEARCH_THRESHOLD = 15

export function taskFamily(task: Task): TaskFamily
export function tasksForTab(tasks: Task[], tab: TaskTab): Task[] // filtered + sorted by byCategory
export function tabCounts(tasks: Task[]): Record<TaskTab, number>
export function matchesTask(task: Task, text: string): boolean // describeTask(task) contains text, case-insensitive

export function describeTask(task: Task): string // extended: + reconcile_plex_collections, humanized fallback
```

```tsx
// components/TasksView.tsx — local state only
const [tab, setTab] = useState<TaskTab>('active')
const [search, setSearch] = useState('')
```

## Call Stack

Render flow (`TasksView`):
1. `useTasks()` → `tasks: Task[]`
2. `tabCounts(tasks)` → counts shown in each tab trigger.
3. `tasksForTab(tasks, tab)` → the current tab's tasks (sorted).
4. if `rows.length > TASK_SEARCH_THRESHOLD`: show search `Input`; when `search`
   non-empty, keep `rows.filter((t) => matchesTask(t, search))`.
5. For each row: `taskFamily(task)` → icon; `describeTask(task)` → sentence;
   `taskCategory(task)` → badge; `task.last_error` → muted second line with a
   `Tooltip` for the full text.
6. Empty states: `tab === 'active'` and no rows → "Nothing running right now";
   search non-empty and no matches → "Nothing matches".
7. Changing tab (`setTab`) also clears `search`.

## Test Plan

Group 1 — pure-logic unit tests (`lib/tasks.test.ts`), each one red-green cycle:
1. `describeTask` describes a `reconcile_plex_collections` task in plain language.
2. `describeTask` describes an `update_ytdlp` task in plain language.
3. `describeTask` never returns a raw snake_case string for an unknown type (humanized fallback).
4. `taskFamily` maps each task type to its family (downloads/syncs/cleanup/maintenance).
5. `tasksForTab('active', …)` returns only `running` tasks, any type.
6. `tasksForTab` returns the right types for `downloads`, `syncs`, and `cleanup`.
7. `tasksForTab('all', …)` returns every task, sorted by `byCategory`.
8. A `maintenance` task appears in `all` and (when running) `active`, but not in the other family tabs.
9. `tabCounts` returns counts equal to each tab's listed length.
10. `matchesTask` matches on the description, case-insensitively.

Group 2 — component-behaviour tests (`TasksView.test.tsx`), each one red-green cycle:
1. The view opens on the Active tab, listing only running tasks.
2. Each tab trigger shows its task count.
3. Selecting Downloads lists download/thumbnail tasks and no sync/cleanup tasks.
4. An empty Active tab shows "Nothing running right now" while other tabs still list tasks.
5. The search field is hidden at ≤15 tasks and shown above 15.
6. Typing in search filters the current tab by description; a non-matching term shows "Nothing matches".
7. Switching tabs clears the search text.
8. A retried task's `last_error` is shown on its row; a task with no error shows none.
