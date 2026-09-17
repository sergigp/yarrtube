import { usePolling } from '../usePolling'
import { fetchTasks } from '../api'
import { formatRelativeTime } from '../formatDateTime'
import { Beacon } from './Beacon'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

function describeTask(task) {
  const payload = task.payload ?? {}

  switch (task.task_type) {
    case 'reconcile_playlist':
      return `Syncing playlist ${payload.playlist_name ?? 'an unknown playlist'}`
    case 'reconcile_channel':
      return `Syncing channel ${payload.channel_name ?? 'an unknown channel'}`
    case 'download_video': {
      const video = payload.video_title ?? 'a video'
      const container = payload.playlist_name ?? payload.channel_name ?? 'an unknown playlist or channel'
      return `Downloading ${video} in ${container}`
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

// Running tasks are worth noticing first, then tasks already due to run
// ("queued"), then tasks still scheduled for later.
const CATEGORY_ORDER = { running: 0, queued: 1, pending: 2 }

function taskCategory(task) {
  if (task.status !== 'pending') {
    return task.status
  }
  return new Date(task.run_at).getTime() <= Date.now() ? 'queued' : 'pending'
}

function byCategory(a, b) {
  const diff = (CATEGORY_ORDER[taskCategory(a)] ?? 3) - (CATEGORY_ORDER[taskCategory(b)] ?? 3)
  if (diff !== 0) {
    return diff
  }
  return new Date(a.run_at).getTime() - new Date(b.run_at).getTime()
}

const CATEGORY_BADGE_VARIANT = {
  running: 'default',
  queued: 'outline',
  pending: 'secondary',
}

export function TasksView() {
  const { data: tasks, error } = usePolling(fetchTasks, [])

  if (error) {
    return <p className="text-sm text-destructive">Failed to load tasks: {error.message}</p>
  }

  if (!tasks) {
    return <p className="text-sm text-muted-foreground">Loading tasks…</p>
  }

  if (tasks.length === 0) {
    return <p className="text-sm text-muted-foreground">No pending or in-progress tasks.</p>
  }

  const sortedTasks = [...tasks].sort(byCategory)

  return (
    <ul className="flex flex-col divide-y divide-border">
      {sortedTasks.map((task, index) => {
        const category = taskCategory(task)
        return (
          <li
            key={task.id}
            className={cn(
              'flex items-center gap-3 px-2 py-3 animate-enter',
              category === 'running' && 'bg-accent',
            )}
            style={{ animationDelay: `${Math.min(index, 12) * 20}ms` }}
          >
            {category === 'running' && <Beacon variant="live" label="Running" />}
            <span className="min-w-0 flex-1 truncate text-sm text-foreground">
              {describeTask(task)}
            </span>
            <span className="flex shrink-0 items-center gap-1.5">
              <Badge variant={CATEGORY_BADGE_VARIANT[category] ?? 'secondary'}>{category}</Badge>
              {task.retries > 0 && <Badge variant="outline">{task.retries} retries</Badge>}
              <span className="w-16 text-right text-xs text-muted-foreground">
                {category === 'pending' ? formatRelativeTime(task.run_at) : ''}
              </span>
            </span>
          </li>
        )
      })}
    </ul>
  )
}
