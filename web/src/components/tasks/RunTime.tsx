import { formatRunTime } from '@/lib/taskTimes'

/** When a task runs: the clock time, then how long until then. */
export function RunTime({ runAt, now }: { runAt: string; now: Date }) {
  const { clock, relative } = formatRunTime(runAt, now)
  return (
    <span className="flex shrink-0 items-baseline gap-1.5 text-xs text-muted-foreground tabular-nums sm:w-36">
      <span className="font-medium text-foreground">{clock}</span>
      <span aria-hidden>·</span>
      <span>{relative}</span>
    </span>
  )
}
