import type { PlexSync } from '@/api/types'

/** The Plex Media Server version without its build suffix. */
export function plexVersionLabel(version: string): string {
  return version.split('-')[0]
}

/** Whether `sync` is running, or was requested at `requestedAt` and hasn't finished since. */
export function isSyncing(sync: PlexSync, requestedAt: Date | null): boolean {
  if (sync.running) return true
  if (requestedAt === null) return false
  return sync.last_run_at === null || new Date(sync.last_run_at) < requestedAt
}

const RELATIVE_UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['day', 24 * 60 * 60],
  ['hour', 60 * 60],
  ['minute', 60],
  ['second', 1],
]

/** `at` (an ISO timestamp) relative to now: "2 minutes ago", "in 3 minutes". */
export function relativeTime(at: string, now: Date = new Date()): string {
  const seconds = Math.round((new Date(at).getTime() - now.getTime()) / 1000)
  const [unit, size] =
    RELATIVE_UNITS.find(([, unitSeconds]) => Math.abs(seconds) >= unitSeconds) ??
    RELATIVE_UNITS[RELATIVE_UNITS.length - 1]!
  return new Intl.RelativeTimeFormat('en', { numeric: 'auto' }).format(
    Math.round(seconds / size),
    unit,
  )
}

/** A sync's last and next run, or its failure and retry, on one line. */
export function syncStatusLine(sync: PlexSync, now: Date = new Date()): string {
  if (sync.last_error && sync.last_run_at) {
    const failed = `Failed ${relativeTime(sync.last_run_at, now)}: ${sync.last_error}.`
    return sync.next_run_at ? `${failed} Retries ${relativeTime(sync.next_run_at, now)}.` : failed
  }
  const last = sync.last_run_at ? `Last run ${relativeTime(sync.last_run_at, now)}` : 'Not run yet'
  return sync.next_run_at ? `${last} · Next ${relativeTime(sync.next_run_at, now)}` : last
}
