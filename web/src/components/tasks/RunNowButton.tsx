import { Button } from '../ui/button'
import { useTaskAction } from './useTaskAction'

interface RunNowButtonProps {
  label: string
  action: () => Promise<void>
}

/** Asks for tasks to run now, disabled while asking, saying so if it failed. */
export function RunNowButton({ label, action }: RunNowButtonProps) {
  const { run, working, error } = useTaskAction(action)
  return (
    <span className="flex shrink-0 items-center gap-2">
      {error && <span className="text-xs text-destructive">
          {label} failed: {error}
        </span>}
      <Button
        variant="outline"
        size="sm"
        className="min-h-11 sm:min-h-7"
        disabled={working}
        onClick={run}
      >
        {label}
      </Button>
    </span>
  )
}
