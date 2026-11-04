# Design

## Files

- `web/src/lib/tasks.ts`: the AI family, tab and kind; which tabs show.
- `web/src/lib/tasks.test.ts`: family, tab and kind tests.
- `web/src/api/queries.ts`: `useAiEnabled`, next to `useSummariesAvailable`.
- `web/src/lib/preferences.ts`: `aiEnabled`, next to `summariesAvailable`.
- `web/src/components/TasksView.tsx`: render the shown tabs; AI label and icons.
- `web/src/components/TasksView.test.tsx`: AI tab tests.

## Types & Signatures

```ts
export type TaskFamily = 'downloads' | 'syncs' | 'ai' | 'other'
export type TaskTab = 'active' | 'downloads' | 'syncs' | 'ai' | 'other'
export type TaskKind = 'download' | 'sync' | 'delete' | 'ai' | 'maintenance'

/** The tabs shown, in order; "ai" only while `aiEnabled`. */
export function taskTabs(aiEnabled: boolean): readonly TaskTab[]

/** `summarize_video` is 'ai' only while `aiEnabled`, else 'other'. */
export function taskFamily(task: Task, aiEnabled: boolean): TaskFamily

export function tasksForTab(tasks: Task[], tab: TaskTab, aiEnabled: boolean): Task[]

export function tabCounts(tasks: Task[], aiEnabled: boolean): Record<TaskTab, number>

// lib/preferences.ts
export function aiEnabled(preferences: Preferences | undefined): boolean

// api/queries.ts
export function useAiEnabled(): boolean
```

`TASK_TABS` is replaced by `taskTabs`.

## Call Stack

```
TaskTabs()
  -> useTasks(), useAiEnabled()
       -> aiEnabled(usePreferences().data)
  -> tabCounts(tasks, aiEnabled)
       -> tasksForTab(tasks, tab, aiEnabled) -> taskFamily(task, aiEnabled)
  -> taskTabs(aiEnabled).map(TabsTrigger)
  -> tasksForTab(tasks, tab, aiEnabled)
  -> rows: KIND_ICON[taskKind(task)]
```

## Test Plan

1. Behaviour tests

   `web/src/lib/tasks.test.ts`:

   - `puts video summaries in the ai family while AI is on`.
   - `puts video summaries in the other family while AI is off`.
   - `lists the ai tab between syncs and other only while AI is on`.
   - `counts every task once across tabs while AI is on`.
   - `gives video summaries the ai kind`.

   `web/src/components/TasksView.test.tsx`, with `GET /api/preferences` routed:

   - `lists video summaries under AI while AI is on`.
   - `shows an AI tab with no tasks while AI is on`: count 0.
   - `shows no AI tab and lists summaries under Other while AI is off`.
   - `shows no AI tab when preferences fail to load`.

2. Infrastructure tests

   None: no adapter changes.
