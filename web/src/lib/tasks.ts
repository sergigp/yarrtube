import type { Task } from '@/api/types'

/** The video a task targets and its playlist or channel, with placeholders for unresolved names. */
function videoInContainer(payload: NonNullable<Task['payload']>): string {
  const video = payload.video_title ?? 'a video'
  const container =
    payload.playlist_name ?? payload.channel_name ?? 'an unknown playlist or channel'
  return `${video} in ${container}`
}

/** A short human sentence for what a queued task will do. */
export function describeTask(task: Task): string {
  const payload = task.payload ?? {}

  switch (task.task_type) {
    case 'reconcile_playlist':
      return `Syncing playlist ${payload.playlist_name ?? 'an unknown playlist'}`
    case 'reconcile_channel':
      return `Syncing channel ${payload.channel_name ?? 'an unknown channel'}`
    case 'download_video':
      return `Downloading ${videoInContainer(payload)}`
    case 'fetch_thumbnail':
      return `Fetching thumbnail of ${videoInContainer(payload)}`
    case 'fetch_subtitles':
      return `Fetching subtitles of ${videoInContainer(payload)}`
    case 'fetch_chapters':
      return `Fetching chapters of ${videoInContainer(payload)}`
    case 'summarize_video':
      return `Summarizing ${videoInContainer(payload)}`
    case 'upgrade_quality':
      return `Checking the quality of ${videoInContainer(payload)}`
    case 'sync_video':
      return `Syncing ${videoInContainer(payload)}`
    case 'sync_plex_labels':
      return 'Syncing Plex category labels'
    case 'delete_video_file':
      return payload.filename ? `Removing file ${payload.filename}` : 'Removing a video file'
    default:
      if (payload.path) {
        return `Deleting files at ${payload.path}`
      }
      if (task.task_type === 'update_ytdlp') {
        return 'Updating yt-dlp'
      }
      return humanizeType(task.task_type)
  }
}

/** Turns an unknown raw task type into a readable sentence, never snake_case. */
function humanizeType(type: string): string {
  const words = type.replace(/_/g, ' ').trim()
  return words ? words.charAt(0).toUpperCase() + words.slice(1) : 'Task'
}

export type TaskFamily = 'downloads' | 'syncs' | 'ai' | 'other'
export type TaskTab = 'active' | 'downloads' | 'syncs' | 'ai' | 'other'

/** The tabs shown, in order; "ai" only while `aiEnabled`. */
export function taskTabs(aiEnabled: boolean): readonly TaskTab[] {
  return aiEnabled
    ? ['active', 'downloads', 'syncs', 'ai', 'other']
    : ['active', 'downloads', 'syncs', 'other']
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
}

/** A task's family; AI tasks are 'other' unless `aiEnabled`. */
export function taskFamily(task: Task, aiEnabled: boolean): TaskFamily {
  const family = FAMILY_BY_TYPE[task.task_type] ?? 'other'
  return family === 'ai' && !aiEnabled ? 'other' : family
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
