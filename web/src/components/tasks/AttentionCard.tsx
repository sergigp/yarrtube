import { useState } from 'react'
import { TriangleAlert } from 'lucide-react'
import type { Task } from '@/api/types'
import { describeTask } from '@/lib/tasks'
import { explainTaskError } from '@/lib/taskErrors'
import { formatRunTime } from '@/lib/taskTimes'
import { Button } from '../ui/button'

interface AttentionCardProps {
  task: Task
  now: Date
}

/** A failing task: what it does, why it failed in plain words and when it runs again. */
export function AttentionCard({ task, now }: AttentionCardProps) {
  const [showError, setShowError] = useState(false)
  const raw = task.last_error ?? ''

  return (
    <li className="flex flex-col gap-2 border-l-4 border-l-amber-500 bg-amber-500/5 px-3 py-3">
      <span className="flex items-start gap-2">
        <TriangleAlert className="mt-0.5 size-4 shrink-0 text-amber-500" aria-hidden />
        <span className="flex min-w-0 flex-col gap-1">
          <span className="text-sm font-medium text-foreground">{describeTask(task)}</span>
          <span className="text-sm text-muted-foreground">
            {explainTaskError(raw)}. {nextTry(task, now)}
          </span>
        </span>
      </span>
      <span className="flex flex-wrap items-center gap-2 sm:pl-6">
        <Button
          variant="ghost"
          size="sm"
          className="min-h-11 sm:min-h-7"
          aria-expanded={showError}
          onClick={() => setShowError(!showError)}
        >
          {showError ? 'Hide error' : 'Show error'}
        </Button>
      </span>
      {showError && (
        <pre className="overflow-x-auto rounded-md bg-muted px-3 py-2 text-xs whitespace-pre-wrap text-muted-foreground sm:ml-6">
          {raw}
        </pre>
      )}
    </li>
  )
}

/** When Yarrtube tries the task again. */
function nextTry(task: Task, now: Date): string {
  const { clock, relative } = formatRunTime(task.run_at, now)
  return relative === 'Starting soon'
    ? 'Yarrtube tries again soon.'
    : `Yarrtube tries again at ${clock}, ${relative}.`
}
