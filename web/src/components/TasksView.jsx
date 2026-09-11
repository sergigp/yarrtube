import { usePolling } from '../usePolling'
import { fetchTasks } from '../api'

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

  return (
    <ul className="list">
      {tasks.map((task) => (
        <li key={task.id} className="list-item">
          <span className="list-item-title">{task.task_type}</span>
          <span className="list-item-meta">
            {task.status} · {task.retries} retries
          </span>
        </li>
      ))}
    </ul>
  )
}
