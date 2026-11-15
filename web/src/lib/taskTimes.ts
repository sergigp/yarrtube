export interface RunTime {
  /** The clock time, prefixed with the day when not today, e.g. "20:20" or "Tomorrow 09:10". */
  clock: string
  /** e.g. "in 17 min", or "Starting soon" once due. */
  relative: string
  /** Whether the run time has come. */
  due: boolean
}

/** The browser's locale and time zone unless given, which tests pin. */
export interface TimeFormat {
  locale?: string
  timeZone?: string
}

/** The relative time of a task that is due. */
export const STARTING_SOON = 'Starting soon'

const MINUTE_MS = 60 * 1000
const DAY_MS = 24 * 60 * MINUTE_MS

/** When a task runs, as a clock time and how long until then. */
export function formatRunTime(runAt: string, now: Date, format: TimeFormat = {}): RunTime {
  const at = new Date(runAt)
  const remainingMs = at.getTime() - now.getTime()
  const due = remainingMs <= 0
  return {
    clock: `${dayPrefix(at, now, format)}${clockTime(at, format)}`,
    relative: due ? STARTING_SOON : `in ${formatSpan(remainingMs)}`,
    due,
  }
}

/** When Yarrtube tries a failing task again: "soon" once due, else the time. */
export function formatNextTry(runAt: string, now: Date, format: TimeFormat = {}): string {
  const { clock, relative, due } = formatRunTime(runAt, now, format)
  return due
    ? 'Yarrtube tries again soon.'
    : `Yarrtube tries again at ${clock}, ${relative}.`
}

/** How long ago a running task started, e.g. "started 12 s ago". */
export function formatStartedAgo(startedAt: string, now: Date): string {
  const elapsedMs = Math.max(0, now.getTime() - new Date(startedAt).getTime())
  if (elapsedMs < MINUTE_MS) {
    return `started ${Math.floor(elapsedMs / 1000)} s ago`
  }
  if (elapsedMs < 60 * MINUTE_MS) {
    return `started ${Math.floor(elapsedMs / MINUTE_MS)} min ago`
  }
  return `started ${Math.floor(elapsedMs / (60 * MINUTE_MS))} h ago`
}

/** A span rounded up to the minute: "17 min", "2 h", "2 h 10 min". */
function formatSpan(ms: number): string {
  const minutes = Math.ceil(ms / MINUTE_MS)
  if (minutes < 60) {
    return `${minutes} min`
  }
  const hours = Math.floor(minutes / 60)
  const rest = minutes % 60
  return rest === 0 ? `${hours} h` : `${hours} h ${rest} min`
}

function clockTime(at: Date, { locale, timeZone }: TimeFormat): string {
  return new Intl.DateTimeFormat(locale, { hour: '2-digit', minute: '2-digit', timeZone }).format(at)
}

/** "" today, "Tomorrow " tomorrow, else the weekday, e.g. "Mon ". */
function dayPrefix(at: Date, now: Date, format: TimeFormat): string {
  const day = dayKey(at, format)
  if (day === dayKey(now, format)) {
    return ''
  }
  if (day === dayKey(new Date(now.getTime() + DAY_MS), format)) {
    return 'Tomorrow '
  }
  const weekday = new Intl.DateTimeFormat(format.locale, {
    weekday: 'short',
    timeZone: format.timeZone,
  }).format(at)
  return `${weekday} `
}

/** The calendar day of `date` in the time zone, as YYYY-MM-DD. */
function dayKey(date: Date, { timeZone }: TimeFormat): string {
  return new Intl.DateTimeFormat('en-CA', { timeZone }).format(date)
}
