import type { Task } from '@/api/types'

/** A short human sentence for what a queued task will do. */
export function describeTask(task: Task): string {
  const payload = task.payload ?? {}

  switch (task.task_type) {
    case 'reconcile_playlist':
      return `Syncing playlist ${payload.playlist_name ?? 'an unknown playlist'}`
    case 'reconcile_channel':
      return `Syncing channel ${payload.channel_name ?? 'an unknown channel'}`
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
      return task.task_type
  }
}

export type TaskFamily = 'downloads' | 'syncs' | 'cleanup' | 'maintenance'
export type TaskTab = 'active' | 'downloads' | 'syncs' | 'cleanup' | 'all'

export const TASK_TABS: readonly TaskTab[] = ['active', 'downloads', 'syncs', 'cleanup', 'all']

/** Above this many tasks in a tab, the search field is worth showing. */
export const TASK_SEARCH_THRESHOLD = 15

const FAMILY_BY_TYPE: Record<string, TaskFamily> = {
  download_video: 'downloads',
  fetch_thumbnail: 'downloads',
  reconcile_playlist: 'syncs',
  reconcile_channel: 'syncs',
  reconcile_plex_collections: 'syncs',
  delete_video_file: 'cleanup',
  delete_playlist_files: 'cleanup',
  delete_channel_files: 'cleanup',
}

export function taskFamily(task: Task): TaskFamily {
  return FAMILY_BY_TYPE[task.task_type] ?? 'maintenance'
}

export function tasksForTab(tasks: Task[], tab: TaskTab): Task[] {
  const inTab =
    tab === 'all'
      ? tasks
      : tab === 'active'
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
