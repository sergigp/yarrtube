import { usePolling } from '../usePolling'
import { fetchTasks } from '../api'
import { formatRelativeTime } from '../formatDateTime'

function describeTask(task) {
  const payload = task.payload ?? {}

  switch (task.task_type) {
    case 'reconcile_playlist':
      return `Reconciling playlist ${payload.playlist_name ?? 'an unknown playlist'}`
    case 'reconcile_channel':
      return `Reconciling channel ${payload.channel_name ?? 'an unknown channel'}`
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

// Running tasks are the ones worth noticing at a glance, so they sort first.
const STATUS_ORDER = { running: 0, pending: 1 }

function byStatus(a, b) {
  return (STATUS_ORDER[a.status] ?? 2) - (STATUS_ORDER[b.status] ?? 2)
}

export function TasksView() {
  const { data: tasks, error } = usePolling(fetchTasks, [])

  if (error) {
    return <p className="error">Failed to load tasks: {error.message}</p>
  }

  if (!tasks) {
    return <p className="muted">Loading tasks…</p>
  }

  if (tasks.length === 0) {
    return <p className="muted">No pending or in-progress tasks.</p>
  }

  const sortedTasks = [...tasks].sort(byStatus)

  return (
    <ul className="list">
      {sortedTasks.map((task) => (
        <li key={task.id} className="list-item">
          <span className="list-item-title">{describeTask(task)}</span>
          <span className="chip-group">
            <span className={`status-badge status-badge-${task.status}`}>{task.status}</span>
            {task.retries > 0 && (
              <span className="status-badge">{task.retries} retries</span>
            )}
            <span className="list-item-meta">{formatRelativeTime(task.run_at)}</span>
          </span>
        </li>
      ))}
    </ul>
  )
}
