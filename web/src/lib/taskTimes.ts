export interface RunTime {
  /** The clock time, e.g. "20:20". */
  clock: string
  /** e.g. "in 17 min", or "Starting soon" once due. */
  relative: string
}

/** When a task runs, as a clock time and how long until then. */
export function formatRunTime(_runAt: string, _now: Date, _locale?: string): RunTime {
  return { clock: '', relative: '' }
}

/** How long ago a running task started, e.g. "started 12 s ago". */
export function formatStartedAgo(_startedAt: string, _now: Date): string {
  return ''
}
