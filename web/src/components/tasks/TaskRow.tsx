import { useId } from 'react'
import { runTasksNow } from '@/api/client'
import type { Task } from '@/api/types'
import { describeTask, offersRunNow } from '@/lib/tasks'
import { formatStartedAgo } from '@/lib/taskTimes'
import { Beacon } from '../Beacon'
import { RunNowButton } from './RunNowButton'
import { RunTime } from './RunTime'

interface TaskRowProps {
  task: Task
  now: Date
}

/** One scheduled or running task: when, then what. */
export function TaskRow({ task, now }: TaskRowProps) {
  const descriptionId = useId()
  return (
    <li className="flex flex-col gap-1 px-3 py-3 sm:flex-row sm:items-center sm:gap-4">
      <RowTime task={task} now={now} />
      <span id={descriptionId} className="min-w-0 flex-1 truncate text-sm text-foreground">
        {describeTask(task)}
      </span>
      {offersRunNow(task) && (
        <RunNowButton
          label="Run now"
          action={() => runTasksNow([task.id])}
          describedBy={descriptionId}
        />
      )}
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
  return <RunTime runAt={task.run_at} now={now} />
}
