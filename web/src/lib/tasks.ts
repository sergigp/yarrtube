import type { Task } from '@/api/types'

/** A short human sentence for what a queued task will do. */
export function describeTask(task: Task): string {
  const payload = task.payload ?? {}

  switch (task.task_type) {
    case 'reconcile_playlist':
      return `Syncing playlist ${payload.playlist_name ?? 'an unknown playlist'}`
    case 'reconcile_channel':
      return `Syncing channel ${payload.channel_name ?? 'an unknown channel'}`
    case 'reconcile_plex_collections':
      return 'Syncing Plex collections'
    case 'download_video': {
      const video = payload.video_title ?? 'a video'
      const container =
        payload.playlist_name ?? payload.channel_name ?? 'an unknown playlist or channel'
      return `Downloading ${video} in ${container}`
    }
    case 'fetch_thumbnail': {
      const video = payload.video_title ?? 'a video'
      const container =
        payload.playlist_name ?? payload.channel_name ?? 'an unknown playlist or channel'
      return `Fetching thumbnail of ${video} in ${container}`
    }
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

export type TaskFamily = 'downloads' | 'syncs' | 'other'
export type TaskTab = 'active' | 'downloads' | 'syncs' | 'other'

export const TASK_TABS: readonly TaskTab[] = ['active', 'downloads', 'syncs', 'other']

/** Above this many tasks in a tab, the search field is worth showing. */
export const TASK_SEARCH_THRESHOLD = 15

const FAMILY_BY_TYPE: Record<string, TaskFamily> = {
  download_video: 'downloads',
  fetch_thumbnail: 'downloads',
  reconcile_playlist: 'syncs',
  reconcile_channel: 'syncs',
}

export function taskFamily(task: Task): TaskFamily {
  return FAMILY_BY_TYPE[task.task_type] ?? 'other'
}

export function tasksForTab(tasks: Task[], tab: TaskTab): Task[] {
  const inTab =
    tab === 'active'
      ? tasks.filter((task) => task.status === 'running')
      : tasks.filter((task) => taskFamily(task) === tab)
  return [...inTab].sort(byCategory)
}

export function tabCounts(tasks: Task[]): Record<TaskTab, number> {
  return TASK_TABS.reduce(
    (counts, tab) => {
      counts[tab] = tasksForTab(tasks, tab).length
      return counts
    },
    {} as Record<TaskTab, number>,
  )
}

export type TaskKind = 'download' | 'sync' | 'delete' | 'maintenance'

const KIND_BY_TYPE: Record<string, TaskKind> = {
  download_video: 'download',
  fetch_thumbnail: 'download',
  reconcile_playlist: 'sync',
  reconcile_channel: 'sync',
  reconcile_plex_collections: 'sync',
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

/** The playlist or channel a Run now on this task would sync, if it offers one. */
export function syncTarget(_task: Task): SyncTarget | null {
  return null
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
