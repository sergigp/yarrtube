import type { Task } from '@/api/types'

/** The video a task targets and its playlist or channel, with placeholders for unresolved names. */
function videoAndSource(payload: Task['payload'], joiner: 'in' | 'from'): string {
  const video = payload.video_title ?? 'a video'
  const source = payload.playlist_name ?? payload.channel_name ?? 'an unknown playlist or channel'
  return `${video} ${joiner} ${source}`
}

/** What a task does, as a verb in its base form and its "-ing" form, then the rest. */
function taskAction(task: Task): [base: string, ing: string, rest: string] | null {
  const payload = task.payload ?? {}
  switch (task.task_type) {
    case 'reconcile_playlist':
      return ['Check', 'Checking', `${payload.playlist_name ?? 'an unknown playlist'} for new videos`]
    case 'reconcile_channel':
      return ['Check', 'Checking', `${payload.channel_name ?? 'an unknown channel'} for new videos`]
    case 'download_video':
      return ['Download', 'Downloading', videoAndSource(payload, 'from')]
    case 'fetch_thumbnail':
      return ['Fetch', 'Fetching', `the thumbnail of ${videoAndSource(payload, 'in')}`]
    case 'fetch_subtitles':
      return ['Fetch', 'Fetching', `the subtitles of ${videoAndSource(payload, 'in')}`]
    case 'fetch_chapters':
      return ['Fetch', 'Fetching', `the chapters of ${videoAndSource(payload, 'in')}`]
    case 'upgrade_quality':
      return ['Check', 'Checking', `the quality of ${videoAndSource(payload, 'in')}`]
    case 'summarize_video':
      return ['Summarize', 'Summarizing', videoAndSource(payload, 'in')]
    case 'categorize_video':
      return ['Categorize', 'Categorizing', videoAndSource(payload, 'in')]
    case 'sync_video':
      return ['Sync', 'Syncing', videoAndSource(payload, 'in')]
    case 'sync_plex_watch_state':
      return ['Sync', 'Syncing', 'watch state with Plex']
    case 'sync_plex_labels':
      return ['Update', 'Updating', 'category labels in Plex']
    case 'update_ytdlp':
      return ['Update', 'Updating', 'yt-dlp']
    case 'delete_video_file':
      return ['Remove', 'Removing', payload.filename ? `the file ${payload.filename}` : 'a video file']
    default:
      return payload.path ? ['Delete', 'Deleting', `the files at ${payload.path}`] : null
  }
}

/** A short sentence for a task: what it will do while pending, what it is doing while running. */
export function describeTask(task: Task): string {
  const action = taskAction(task)
  if (!action) {
    return humanizeType(task.task_type)
  }
  const [base, ing, rest] = action
  return `${task.status === 'running' ? ing : base} ${rest}`
}

/** Turns an unknown raw task type into a readable sentence, never snake_case. */
function humanizeType(type: string): string {
  const words = type.replace(/_/g, ' ').trim()
  return words ? words.charAt(0).toUpperCase() + words.slice(1) : 'Task'
}

export type TaskFamily = 'downloads' | 'syncs' | 'ai' | 'upkeep'
export type TaskTab = 'active' | TaskFamily

/** The tabs shown, in order; "ai" only while `aiEnabled`. */
export function taskTabs(aiEnabled: boolean): readonly TaskTab[] {
  return aiEnabled
    ? ['active', 'downloads', 'syncs', 'ai', 'upkeep']
    : ['active', 'downloads', 'syncs', 'upkeep']
}

/** Above this many tasks in a tab, the search field is worth showing. */
export const TASK_SEARCH_THRESHOLD = 15

const FAMILY_BY_TYPE: Record<string, TaskFamily> = {
  download_video: 'downloads',
  fetch_thumbnail: 'downloads',
  fetch_subtitles: 'downloads',
  fetch_chapters: 'downloads',
  upgrade_quality: 'downloads',
  reconcile_playlist: 'syncs',
  reconcile_channel: 'syncs',
  sync_video: 'syncs',
  summarize_video: 'ai',
  categorize_video: 'ai',
}

/** A task's family; AI tasks are Plex & upkeep unless `aiEnabled`. */
export function taskFamily(task: Task, aiEnabled: boolean): TaskFamily {
  const family = FAMILY_BY_TYPE[task.task_type] ?? 'upkeep'
  return family === 'ai' && !aiEnabled ? 'upkeep' : family
}

export function tasksForTab(tasks: Task[], tab: TaskTab, aiEnabled: boolean): Task[] {
  const inTab =
    tab === 'active'
      ? tasks.filter((task) => task.status === 'running')
      : tasks.filter((task) => taskFamily(task, aiEnabled) === tab)
  return [...inTab].sort(byCategory)
}

export function tabCounts(tasks: Task[], aiEnabled: boolean): Record<TaskTab, number> {
  return taskTabs(aiEnabled).reduce(
    (counts, tab) => {
      counts[tab] = tasksForTab(tasks, tab, aiEnabled).length
      return counts
    },
    { ai: 0 } as Record<TaskTab, number>,
  )
}

export type TaskKind = 'download' | 'sync' | 'delete' | 'ai' | 'maintenance'

const KIND_BY_TYPE: Record<string, TaskKind> = {
  download_video: 'download',
  fetch_thumbnail: 'download',
  fetch_subtitles: 'download',
  fetch_chapters: 'download',
  upgrade_quality: 'download',
  reconcile_playlist: 'sync',
  reconcile_channel: 'sync',
  sync_video: 'sync',
  summarize_video: 'ai',
  categorize_video: 'ai',
  delete_video_file: 'delete',
  delete_playlist_files: 'delete',
  delete_channel_files: 'delete',
}

/** What kind of work a task does, which picks its row icon. */
export function taskKind(task: Task): TaskKind {
  return KIND_BY_TYPE[task.task_type] ?? 'maintenance'
}

export interface SyncTarget {
  kind: 'playlist' | 'channel'
  id: string
}

// Which payload field holds the id each syncable task type targets.
const SYNC_ID_FIELD: Record<string, { kind: SyncTarget['kind']; field: string }> = {
  reconcile_playlist: { kind: 'playlist', field: 'playlist_id' },
  reconcile_channel: { kind: 'channel', field: 'channel_id' },
}

/** The playlist or channel a Run now on this task would sync, if it offers one. */
export function syncTarget(task: Task): SyncTarget | null {
  const target = SYNC_ID_FIELD[task.task_type]
  const id = target && task.payload[target.field]
  if (task.status === 'running' || !id) {
    return null
  }
  return { kind: target.kind, id }
}

export function matchesTask(task: Task, text: string): boolean {
  return describeTask(task).toLowerCase().includes(text.toLowerCase())
}

export type TaskCategory = 'running' | 'queued' | 'pending' | string

// Running tasks are worth noticing first, then tasks already due to run
// ("queued"), then tasks still scheduled for later.
const CATEGORY_ORDER: Record<string, number> = { running: 0, queued: 1, pending: 2 }

export function taskCategory(task: Task): TaskCategory {
  if (task.status !== 'pending') {
    return task.status
  }
  return new Date(task.run_at).getTime() <= Date.now() ? 'queued' : 'pending'
}

export function byCategory(a: Task, b: Task): number {
  const diff = (CATEGORY_ORDER[taskCategory(a)] ?? 3) - (CATEGORY_ORDER[taskCategory(b)] ?? 3)
  if (diff !== 0) {
    return diff
  }
  return new Date(a.run_at).getTime() - new Date(b.run_at).getTime()
}

export type TaskFilter = 'all' | TaskFamily
export type TaskSectionId = 'running' | 'attention' | 'nextHour' | 'later'

export interface BatchSource {
  taskId: number
  name: string
  kind: 'channel' | 'playlist'
  sourceId: string
}

export type TaskEntry =
  | { kind: 'task'; task: Task }
  | { kind: 'batch'; tasks: Task[]; sources: BatchSource[]; runAt: string }

export interface TaskSection {
  id: TaskSectionId
  entries: TaskEntry[]
}

export interface FilterChip {
  filter: TaskFilter
  count: number
}

export interface TaskListModel {
  chips: FilterChip[]
  sections: TaskSection[]
}

/** Whether the task works on one video. */
export function isPerVideoTask(_task: Task): boolean {
  return false
}

/** Whether the task offers a Run now (or Sync now) action. */
export function offersRunNow(_task: Task): boolean {
  return false
}

const HOUR_MS = 60 * 60 * 1000

/** Which section of the list the task belongs in at `now`. */
export function sectionOf(task: Task, now: Date): TaskSectionId {
  if (task.status === 'running') {
    return 'running'
  }
  if (task.last_error) {
    return 'attention'
  }
  return new Date(task.run_at).getTime() - now.getTime() <= HOUR_MS ? 'nextHour' : 'later'
}

/** Reconciles due within this long of the previous one join its batch. */
const BATCH_GAP_MS = 10 * 60 * 1000

/**
 * The tasks in run-time order, pending reconciles without an error that are
 * each due within 10 minutes of the previous one grouped into batches of two
 * or more. A batch takes the place of its first reconcile.
 */
export function batchReconciles(tasks: Task[]): TaskEntry[] {
  const sorted = [...tasks].sort(byRunAt)
  const chains = chainReconciles(sorted.filter(isBatchable))
  const batched = new Set(chains.flat().map((task) => task.id))
  const batchByFirst = new Map(chains.map((chain) => [chain[0].id, chain]))

  return sorted.flatMap((task): TaskEntry[] => {
    const chain = batchByFirst.get(task.id)
    if (chain) {
      return [{ kind: 'batch', tasks: chain, sources: chain.map(batchSource), runAt: task.run_at }]
    }
    return batched.has(task.id) ? [] : [{ kind: 'task', task }]
  })
}

function isBatchable(task: Task): boolean {
  return (
    (task.task_type === 'reconcile_playlist' || task.task_type === 'reconcile_channel') &&
    task.status !== 'running' &&
    !task.last_error
  )
}

/** Runs of reconciles each due within the gap of the previous, of two or more. */
function chainReconciles(reconciles: Task[]): Task[][] {
  const chains = reconciles.reduce<Task[][]>((chains, task) => {
    const last = chains.at(-1)
    const previous = last?.at(-1)
    if (last && previous && runAtMs(task) - runAtMs(previous) <= BATCH_GAP_MS) {
      last.push(task)
    } else {
      chains.push([task])
    }
    return chains
  }, [])
  return chains.filter((chain) => chain.length >= 2)
}

function batchSource(task: Task): BatchSource {
  return task.task_type === 'reconcile_playlist'
    ? {
        taskId: task.id,
        name: task.payload.playlist_name ?? 'an unknown playlist',
        kind: 'playlist',
        sourceId: task.payload.playlist_id ?? '',
      }
    : {
        taskId: task.id,
        name: task.payload.channel_name ?? 'an unknown channel',
        kind: 'channel',
        sourceId: task.payload.channel_id ?? '',
      }
}

function runAtMs(task: Task): number {
  return new Date(task.run_at).getTime()
}

function byRunAt(a: Task, b: Task): number {
  return runAtMs(a) - runAtMs(b)
}

const FAMILIES: readonly TaskFamily[] = ['downloads', 'syncs', 'ai', 'upkeep']
const SECTIONS: readonly TaskSectionId[] = ['running', 'attention', 'nextHour', 'later']

/**
 * The chips and sections the Tasks view shows: a chip per family with tasks,
 * and the selected chip's tasks matching `query` in sections, batched.
 */
export function buildTaskList(
  tasks: Task[],
  options: { filter: TaskFilter; query: string; aiEnabled: boolean; now: Date },
): TaskListModel {
  const { filter, query, aiEnabled, now } = options
  const listed = tasks.filter(
    (task) =>
      (filter === 'all' || taskFamily(task, aiEnabled) === filter) && matchesTask(task, query.trim()),
  )
  const sections = SECTIONS.map((id) => ({
    id,
    entries: batchReconciles(listed.filter((task) => sectionOf(task, now) === id)),
  })).filter((section) => section.entries.length > 0)
  return { chips: filterChips(tasks, aiEnabled), sections }
}

/** All, then each family with tasks, with their counts. */
function filterChips(tasks: Task[], aiEnabled: boolean): FilterChip[] {
  const families = FAMILIES.map((family) => ({
    filter: family,
    count: tasks.filter((task) => taskFamily(task, aiEnabled) === family).length,
  })).filter((chip) => chip.count > 0)
  return [{ filter: 'all', count: tasks.length }, ...families]
}

/** One sentence saying what is happening now. */
export function statusLine(_tasks: Task[], _now: Date): string {
  return ''
}
