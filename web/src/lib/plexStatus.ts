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
