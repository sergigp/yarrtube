import { Button } from '../ui/button'
import { useTaskAction } from './useTaskAction'
import { TASK_BUTTON_SIZE } from './taskButton'

interface RunNowButtonProps {
  label: string
  action: () => Promise<void>
  /** The id of what the button acts on, so screen readers tell rows apart. */
  describedBy: string
}

/** Asks for tasks to run now, disabled while asking, saying so if it failed. */
export function RunNowButton({ label, action, describedBy }: RunNowButtonProps) {
  const { run, working, error } = useTaskAction(action)
  return (
    <span className="flex shrink-0 items-center gap-2">
      {error && <span className="text-xs text-destructive">
          {label} failed: {error}
        </span>}
      <Button
        variant="outline"
        size="sm"
        className={TASK_BUTTON_SIZE}
        disabled={working}
        aria-describedby={describedBy}
        onClick={run}
      >
        {label}
      </Button>
    </span>
  )
}
