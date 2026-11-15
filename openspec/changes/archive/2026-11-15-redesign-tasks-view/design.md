# Design

## Context

- `GET /api/tasks` returns `TaskResponse` built from `TaskView`; context is
  resolved into a `payload: HashMap<String, String>` by `TaskViewSearcher`
  (`playlist_id`, `channel_id`, names, `video_title`). Per-video tasks know
  their `video_id`, but the listing does not expose it.
- `tasks.updated_at` is set when a task is claimed
  (`UPDATE … SET status = 'running', updated_at = ?2`), so a running task's
  `updated_at` is its start time. No migration is needed.
- `ScheduledTask::bring_forward(run_at, now)` already exists and is used by
  `VideoRetrier`.
- Per-video failures are counted on the video's steps (`video-retries`),
  not on the task: prod's failing download shows `retries: 0` with a
  `last_error`. That is expected, and it is why the UI retries the video
  (`POST /api/videos/{id}/retry`) rather than bringing the task forward. A
  step's task that runs before it is due does nothing.
- Design source: https://claude.ai/artifact/36GX5JmSxQNgjTnyxpcirs.

## Goals / Non-Goals

**Goals:**
- Pure, unit-tested list model in `web/src/lib/` (families, sections,
  batches, copy, status line, run times, error text). Components only render
  it.
- Generic task run-now on the backend, used for single tasks and batches.

**Non-Goals:**
- Stopping a video's retries by hand. A Retrying step already becomes Failed
  on its own after 10 retries, and **Retry now** covers the useful case
  ("I fixed something, try now"). A Stop retrying action was built and then
  dropped: the server could only stop steps that were actually Retrying (not
  a categorization, whose give-up needs the AI job's model, nor a summary
  waiting for the AI server, nor a video sync), so on those cards it
  silently did nothing.
- Progress bars: there is no progress data.
- Changing the sidebar's on-demand reconcile, which stays as it is.
- Listing finished or dead-lettered tasks.

## Decisions

- **Run now brings the scheduled task forward** (`POST /api/tasks/run-now`).
  Today's Run now starts a separate on-demand reconcile next to the
  scheduled one. Bringing the scheduled task forward works for every task
  type, keeps one task per source, and lets a batch be one request.
- **`video_id` goes in the payload map**, like `playlist_id`/`channel_id`.
  `started_at` is a top-level field, because it describes the attempt
  rather than the target.
- **Batching is a front-end concern**: reconciles are chained when each is
  ≤10 min after the previous one, in run-time order, and need at least 2.
  This only changes display.
- **Needs attention = pending task with `last_error`.** Waiting-for-Ollama
  tasks also land there. That is fine: their reason is shown in plain words.
- **Times use the browser's locale and time zone**
  (`Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' })`).
  Tests pin them through a `{ locale?, timeZone? }` options object, empty in
  the app; Vitest also pins `TZ=UTC`.
- **A selected chip falls back to All** once it has no tasks left (e.g. the
  last download finished), so a drained family never leaves an empty,
  "nothing matches" view. `buildTaskList` returns the chip it applied.
- **Failing non-video tasks offer Run now on their card**, like any pending
  non-video task (`offersRunNow`); per-video cards offer Retry now instead.
- **Every task button is described by its row**: buttons keep short names
  ("Run now", "Sync now", …) and point `aria-describedby` at the row's task
  description, the batch description or the source name, so screen readers
  tell identical buttons apart.
- **No Stop retrying** (see Non-Goals): video cards offer Retry now and
  Show error only.

## Risks / Trade-offs

- [Run now on 52 reconciles floods the light lane] → it already runs one at
  a time in run-time order, so this is the same load as the scheduled batch,
  just earlier.
- [yt-dlp error wording changes] → unknown text falls back to the raw first
  line, and the raw text is always one click away.
- [Batch boundaries shift as tasks run] → batches are recomputed on every
  poll. A batch only shrinks or splits, and no action targets a stale id
  because run-now ignores ids that are not pending.
- [Removing the Syncs-tab on-demand reconcile] → that endpoint stays, so
  the sidebar is unaffected.

## Diagrams

Colours: green = new, yellow = changed, red = removed, plain = unchanged.

```mermaid
---
title: Types added or changed by the tasks view redesign (backend)
---
classDiagram
  classDef new fill:#d4f5d4,stroke:#2e7d32
  classDef changed fill:#fff1c2,stroke:#b58900
  classDef removed fill:#f8d4d4,stroke:#c62828,stroke-dasharray:4

  class TaskView {
    +started_at: Option~DateTime~
  }
  class TaskViewSearcher
  class TaskRunNowRequester {
    +run_now(ids) Result
  }
  class TaskRepository {
    <<port>>
    +list_pending_by_ids(ids) Vec~ScheduledTask~
  }
  class SqliteTaskRepository
  class ScheduledTask {
    +bring_forward(run_at, now) Self
    +started_at() Option~DateTime~
  }

  TaskViewSearcher --> TaskRepository
  TaskViewSearcher ..> TaskView
  TaskRunNowRequester --> TaskRepository
  TaskRunNowRequester ..> ScheduledTask
  SqliteTaskRepository ..|> TaskRepository

  class TaskView:::changed
  class TaskViewSearcher:::changed
  class TaskRunNowRequester:::new
  class TaskRepository:::changed
  class SqliteTaskRepository:::changed
```

```mermaid
---
title: Web list model for the tasks view
---
classDiagram
  classDef new fill:#d4f5d4,stroke:#2e7d32
  classDef changed fill:#fff1c2,stroke:#b58900

  class TasksView
  class lib_tasks {
    +buildTaskList(tasks, options) TaskListModel
    +statusLine(tasks, now) string
    +describeTask(task) string
    +taskFamily(task, aiEnabled) TaskFamily
    +namedSources(sources) string
  }
  class lib_taskErrors {
    +explainTaskError(raw) string
  }
  class lib_taskTimes {
    +formatRunTime(runAt, now, format) RunTime
    +formatNextTry(runAt, now, format) string
    +formatStartedAgo(startedAt, now) string
  }
  class TaskFilterChips
  class TaskRow
  class TaskBatchRow {
    useChannels() avatars
  }
  class AttentionCard
  class RunNowButton
  class RunTime
  class useTaskAction

  TasksView --> lib_tasks
  TasksView --> TaskFilterChips
  TasksView --> TaskRow
  TasksView --> TaskBatchRow
  TasksView --> AttentionCard
  AttentionCard --> lib_taskErrors
  AttentionCard --> lib_taskTimes
  AttentionCard --> RunNowButton
  TaskRow --> RunTime
  TaskRow --> RunNowButton
  TaskBatchRow --> RunTime
  TaskBatchRow --> RunNowButton
  TaskBatchRow --> useTaskAction
  AttentionCard --> useTaskAction
  RunNowButton --> useTaskAction
  RunTime --> lib_taskTimes

  class TasksView:::changed
  class lib_tasks:::changed
  class lib_taskErrors:::new
  class lib_taskTimes:::new
  class TaskFilterChips:::new
  class TaskRow:::new
  class TaskBatchRow:::new
  class AttentionCard:::new
  class RunNowButton:::new
  class RunTime:::new
  class useTaskAction:::new
```

```mermaid
---
title: Run tasks now
---
sequenceDiagram
  box rgb(212,245,212) New
    participant H as http::tasks::run_tasks_now
    participant R as TaskRunNowRequester
  end
  box rgb(255,241,194) Changed
    participant Repo as SqliteTaskRepository
  end
  participant UI as Tasks view
  UI->>H: POST /api/tasks/run-now {ids}
  alt ids empty or missing
    H-->>UI: 400
  else
    H->>R: run_now(&ids)
    R->>Repo: list_pending_by_ids(&ids)
    Repo-->>R: Vec<ScheduledTask>
    loop each pending task
      R->>Repo: update(task.bring_forward(now, now))
    end
    alt repository error
      R-->>H: Err
      H-->>UI: 500
    else
      H-->>UI: 204
    end
  end
```

```mermaid
---
title: List tasks (changed fields)
---
sequenceDiagram
  box rgb(255,241,194) Changed
    participant S as TaskViewSearcher
    participant D as TaskResponse
  end
  box Unchanged
    participant H as http::tasks::list_tasks
    participant TR as TaskRepository
  end
  H->>S: search_all_pending()
  S->>TR: list_non_completed()
  TR-->>S: Vec<ScheduledTask>
  S->>S: to_view(task): payload += video_id, started_at = updated_at if running
  S-->>H: Vec<TaskView>
  H->>D: TaskResponse::from(view)
```

## Types & Signatures

```rust
// src/domain/task/task_view.rs
pub struct TaskView {
    pub id: i64,
    pub task_type: String,
    pub status: TaskStatus,
    pub retries: i64,
    pub run_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>, // new: updated_at while running
    pub last_error: Option<String>,
    pub payload: HashMap<String, String>, // + "video_id" for per-video tasks
}
```

```rust
// src/application/http/tasks/dto.rs
pub struct TaskResponse {
    // … existing fields …
    pub started_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct RunTasksNowRequest {
    #[serde(default)]
    pub ids: Option<Vec<i64>>, // missing or empty → 400 MISSING_TASK_IDS
}

// src/application/http/validation.rs
pub const MISSING_TASK_IDS: &str = "At least one task id must be given (missing)";
```

```rust
// src/domain/task/scheduled_task.rs
impl ScheduledTask {
    /// When the running attempt started (`updated_at`), None unless running.
    pub fn started_at(&self) -> Option<DateTime<Utc>>;
}
```

```rust
// src/infrastructure/repositories/sqlite_task_repository.rs
pub trait TaskRepository: Send + Sync {
    // … existing …
    /// The pending tasks among `ids`; running or unknown ids are skipped.
    fn list_pending_by_ids(&self, ids: &[i64]) -> anyhow::Result<Vec<ScheduledTask>>;
}
```

```rust
// src/domain/services/task_run_now_requester.rs
#[derive(Clone)]
pub struct TaskRunNowRequester {
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

pub trait TaskRunNowRequesterApi: Send + Sync {
    /// Each pending task among `ids` becomes due now; others are ignored.
    fn run_now(&self, ids: &[i64]) -> anyhow::Result<()>;
}
```

```rust
// src/application/http/tasks/mod.rs
pub async fn run_tasks_now(
    State(requester): State<TaskRunNowRequester>,
    Json(request): Json<RunTasksNowRequest>,
) -> Result<StatusCode, ApiError>;

// src/application/http/mod.rs
//   .route("/tasks/run-now", post(tasks::run_tasks_now))
//   ApiServices += task_run_now_requester
```

```ts
// web/src/api/types.ts
export interface Task {
  // … existing …
  started_at: string | null
  payload: Record<string, string | undefined> // + video_id
}

// web/src/api/client.ts
export function runTasksNow(ids: number[]): Promise<void>
// retryVideo(videoId) already exists
```

```ts
// web/src/lib/tasks.ts
export type TaskFamily = 'downloads' | 'syncs' | 'ai' | 'upkeep'
export type TaskFilter = 'all' | TaskFamily
export type TaskSectionId = 'running' | 'attention' | 'nextHour' | 'later'

export interface BatchSource { taskId: number; name: string; kind: 'channel' | 'playlist'; sourceId: string }
export type TaskEntry =
  | { kind: 'task'; task: Task }
  | { kind: 'batch'; tasks: Task[]; sources: BatchSource[]; runAt: string }
export interface TaskSection { id: TaskSectionId; entries: TaskEntry[] }
export interface FilterChip { filter: TaskFilter; count: number }
export interface TaskListModel {
  chips: FilterChip[]
  filter: TaskFilter // the chip applied: the one asked for, or 'all' once it is gone
  sections: TaskSection[]
}
export const NAMED_SOURCES = 3
export function namedSources(sources: BatchSource[]): string // "A, B, C and 13 more"

export function taskFamily(task: Task, aiEnabled: boolean): TaskFamily
export function isPerVideoTask(task: Task): boolean
export function offersRunNow(task: Task): boolean
export function describeTask(task: Task): string // base form if pending, "-ing" if running
export function sectionOf(task: Task, now: Date): TaskSectionId
export function batchReconciles(tasks: Task[]): TaskEntry[]
export function buildTaskList(
  tasks: Task[],
  options: { filter: TaskFilter; query: string; aiEnabled: boolean; now: Date },
): TaskListModel
export function statusLine(tasks: Task[], now: Date): string

// web/src/lib/taskTimes.ts
export interface RunTime { clock: string; relative: string } // relative = STARTING_SOON when due
export interface TimeFormat { locale?: string; timeZone?: string }
export const STARTING_SOON = 'Starting soon'
export function formatRunTime(runAt: string, now: Date, format?: TimeFormat): RunTime
export function formatNextTry(runAt: string, now: Date, format?: TimeFormat): string // "Yarrtube tries again at …"
export function formatStartedAgo(startedAt: string, now: Date): string

// web/src/lib/taskErrors.ts
export function explainTaskError(raw: string): string

// web/src/components/tasks/useTaskAction.ts
/** Runs `action`, tracking working/error, and invalidates the tasks query on success. */
export function useTaskAction(action: () => Promise<void>): {
  run: () => Promise<void>; working: boolean; error: string | null
}

// web/src/components/tasks/RunNowButton.tsx
/** describedBy: id of the row's description, set as aria-describedby. */
export function RunNowButton(props: { label: string; action: () => Promise<void>; describedBy: string })

// web/src/components/tasks/RunTime.tsx
export function RunTime(props: { runAt: string; now: Date })

// web/src/components/tasks/taskButton.ts
export const TASK_BUTTON_SIZE = 'min-h-11 sm:min-h-7'
```

## Call Stack

- **List**: `TasksView` → `useTasks()` (`GET /api/tasks`) + `useAiEnabled()`
  → `buildTaskList(tasks, {filter, query, aiEnabled, now})` →
  `TaskFilterChips` (selected = `list.filter`) / per section `TaskRow` |
  `TaskBatchRow` (+ `useChannels()` for avatars) | `AttentionCard`. Server: `list_tasks` →
  `TaskViewSearcher::search_all_pending()` → `TaskRepository::list_non_completed()`
  → `to_view(task)` (adds `video_id`, `started_at`) → `TaskResponse::from`.
- **Run now / Sync now / Sync all now**: row, card (non-video task) or batch
  → `RunNowButton` / `useTaskAction(() => runTasksNow(ids))` →
  `POST /api/tasks/run-now {ids}` → `run_tasks_now(request)` →
  `required(ids.filter(non-empty), MISSING_TASK_IDS)` (400 if missing or
  empty) →
  `TaskRunNowRequester::run_now(&ids)` →
  `TaskRepository::list_pending_by_ids(&ids)` → per task
  `TaskRepository::update(&task.bring_forward(now, now))` → 204 →
  `invalidateTasks()`.
- **Retry now**: card → `useTaskAction(() => retryVideo(payload.video_id))`
  → existing `POST /api/videos/{id}/retry` → `VideoRetrier::retry` →
  `invalidateTasks()`.

## Test Plan

1. Behaviour tests: Rust HTTP handlers (`src/application/http/*/tests.rs`,
   in-memory fakes)
   1. `it_should_include_video_id_for_a_download_task`: the listed download
      task has `payload.video_id`.
   2. `it_should_include_video_id_when_the_video_no_longer_exists`: the
      thumbnail task of a deleted video still has `video_id` and no
      `video_title`.
   3. `it_should_not_include_video_id_for_a_ytdlp_update`
   4. `it_should_include_started_at_for_a_running_task`: equals the claim
      time.
   5. `it_should_have_null_started_at_for_a_pending_task`
   6. `it_should_bring_a_pending_task_forward_when_run_now`: 204, and
      `run_at` = now.
   7. `it_should_bring_every_listed_task_forward`: 3 reconciles, all due now.
   8. `it_should_ignore_running_and_unknown_ids_when_run_now`: 204, and the
      running task is unchanged.
   9. `it_should_keep_retries_and_last_error_when_run_now`
   10. `it_should_reject_run_now_without_ids`: 400 for `{"ids": []}` and
       for `{}`.
2. Infrastructure tests: `SqliteTaskRepository` (real SQLite)
   1. `it_should_list_only_pending_tasks_among_the_given_ids`: pending,
      running and missing ids → only the pending one.
   2. `it_should_list_nothing_for_an_empty_id_list`
3. Web unit tests (`web/src/lib/*.test.ts`, Vitest, fake timers)
   1. `taskFamily`: downloads, syncs, AI on and off, upkeep, unknown type.
   2. `describeTask`: base form when pending, "-ing" when running, for every
      type, with placeholders when a name is missing.
   3. `sectionOf`: running / attention / next hour (incl. already due) /
      later.
   4. `batchReconciles`: a chain within 10 min forms 1 batch, a gap over
      10 min splits it, a lone reconcile stays a task, errored or running
      reconciles are not batched.
   5. `buildTaskList`: chip counts sum to All, empty chips hidden, filter,
      falls back to All once the selected chip has no tasks left, query on
      description and on a batch's source names, empty sections dropped.
      `namedSources`: one, two, three, and "and N more".
   6. `statusLine`: idle with next due, next already due, downloads running,
      mixed running.
   7. `formatRunTime`: today, tomorrow, already due → "Starting soon";
      `formatNextTry`: soon / at a time; `formatStartedAgo`.
   8. `explainTaskError`: 403, private, age-gated, members-only, unavailable,
      unknown → first line without "ERROR:".
4. Web component tests (`web/src/components/TasksView.test.tsx`, `mockApi`)
   1. Opens on All with the status line and sections for the prod-like
      fixture, and shows no badges.
   2. Selecting a chip filters; a chip with no tasks is hidden.
   3. A batch expands and collapses, and Sync all now posts every id.
   4. Sync now posts one id. Run now on a Plex sync posts its id. A
      per-video task has no Run now.
   5. The attention card shows the plain reason; Show error / Hide error
      toggles the raw text.
   6. Retry now posts to `/api/videos/{id}/retry` and refreshes; a video
      card offers no Stop retrying.
   7. A failed action shows an error and can be used again.
   8. Search filters within the chip, and "nothing matches" shows when
      nothing does.
   9. Empty listing → "All quiet. Nothing is scheduled."
   10. A failing Plex sync's card offers Run now, which posts its id.
   11. When the selected chip's last task goes, All is selected again.
   12. Every task button is described by its task, batch or source
       (`aria-describedby`).
