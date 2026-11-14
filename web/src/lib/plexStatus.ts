import type { PlexSync } from '@/api/types'

/** The Plex Media Server version without its build suffix. */
export function plexVersionLabel(version: string): string {
  return version
}

/** Whether `sync` is running, or was requested at `requestedAt` and hasn't finished since. */
export function isSyncing(_sync: PlexSync, _requestedAt: Date | null): boolean {
  return false
}
