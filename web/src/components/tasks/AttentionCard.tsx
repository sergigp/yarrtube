import { useId, useState } from 'react'
import { TriangleAlert } from 'lucide-react'
import { retryVideo, runTasksNow } from '@/api/client'
import type { Task } from '@/api/types'
import { describeTask, isPerVideoTask } from '@/lib/tasks'
import { explainTaskError } from '@/lib/taskErrors'
import { formatNextTry } from '@/lib/taskTimes'
import { Button } from '../ui/button'
import { RunNowButton } from './RunNowButton'
import { useTaskAction } from './useTaskAction'
import { TASK_BUTTON_SIZE } from './taskButton'

interface AttentionCardProps {
  task: Task
  now: Date
}

/**
 * A failing task: what it does, why it failed in plain words and when it runs
 * again. A video's task retries the video; any other runs now.
 */
export function AttentionCard({ task, now }: AttentionCardProps) {
  const [showError, setShowError] = useState(false)
  const descriptionId = useId()
  const raw = task.last_error ?? ''

  return (
    <li className="flex flex-col gap-2 border-l-4 border-l-amber-500 bg-amber-500/5 px-3 py-3">
      <span className="flex items-start gap-2">
        <TriangleAlert className="mt-0.5 size-4 shrink-0 text-amber-500" aria-hidden />
        <span className="flex min-w-0 flex-col gap-1">
          <span id={descriptionId} className="text-sm font-medium text-foreground">
            {describeTask(task)}
          </span>
          <span className="text-sm text-muted-foreground">
            {explainTaskError(raw)}. {formatNextTry(task.run_at, now)}
          </span>
        </span>
      </span>
      <span className="flex flex-wrap items-center gap-2 sm:pl-6">
        {isPerVideoTask(task) ? (
          task.payload.video_id && (
            <RetryButton videoId={task.payload.video_id} describedBy={descriptionId} />
          )
        ) : (
          <RunNowButton
            label="Run now"
            action={() => runTasksNow([task.id])}
            describedBy={descriptionId}
          />
        )}
        <Button
          variant="ghost"
          size="sm"
          className={TASK_BUTTON_SIZE}
          aria-expanded={showError}
          aria-describedby={descriptionId}
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

/** Retry now for the card's video, disabled while it asks. */
function RetryButton({ videoId, describedBy }: { videoId: string; describedBy: string }) {
  const retry = useTaskAction(() => retryVideo(videoId))
  return (
    <>
      <Button
        variant="outline"
        size="sm"
        className={TASK_BUTTON_SIZE}
        disabled={retry.working}
        aria-describedby={describedBy}
        onClick={retry.run}
      >
        Retry now
      </Button>
      {retry.error && <span className="text-xs text-destructive">Retry failed: {retry.error}</span>}
    </>
  )
}
