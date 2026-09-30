import { useTasks } from '@/api/queries'
import { formatRelativeTime } from '@/lib/formatDateTime'
import { byCategory, describeTask, taskCategory } from '@/lib/tasks'
import { Beacon } from './Beacon'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

const CATEGORY_BADGE_VARIANT: Record<string, 'default' | 'outline' | 'secondary'> = {
  running: 'default',
  queued: 'outline',
  pending: 'secondary',
}

export function TasksView() {
  const { data: tasks, error } = useTasks()

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
