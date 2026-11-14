import type { Task } from '@/api/types'
import { describeTask } from '@/lib/tasks'
import { formatRunTime, formatStartedAgo } from '@/lib/taskTimes'
import { Beacon } from '../Beacon'

interface TaskRowProps {
  task: Task
  now: Date
}

/** One scheduled or running task: when, then what. */
export function TaskRow({ task, now }: TaskRowProps) {
  return (
    <li className="flex flex-col gap-1 px-3 py-3 sm:flex-row sm:items-center sm:gap-4">
      <RowTime task={task} now={now} />
      <span className="min-w-0 flex-1 truncate text-sm text-foreground">{describeTask(task)}</span>
    </li>
  )
}

function RowTime({ task, now }: TaskRowProps) {
  if (task.status === 'running') {
    return (
      <span className="flex shrink-0 items-center gap-2 text-xs text-muted-foreground sm:w-36">
        <Beacon variant="live" label="Running" />
        {task.started_at ? formatStartedAgo(task.started_at, now) : 'started'}
      </span>
    )
  }
  const { clock, relative } = formatRunTime(task.run_at, now)
  return (
    <span className="flex shrink-0 items-baseline gap-1.5 text-xs text-muted-foreground tabular-nums sm:w-36">
      <span className="font-medium text-foreground">{clock}</span>
      <span aria-hidden>·</span>
      <span>{relative}</span>
    </span>
  )
}
