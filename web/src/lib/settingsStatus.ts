import type { PlexStatus, Preferences } from '@/api/types'

/** The AI section's status in the Settings nav, or `null` while unknown. */
export function aiStatus(_preferences: Preferences | undefined): string | null {
  return null
}

/** The Plex section's status in the Settings nav, or `null` while unknown. */
export function plexStatus(_status: PlexStatus | undefined): string | null {
  return null
}
