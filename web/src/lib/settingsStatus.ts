import type { PlexStatus, Preferences } from '@/api/types'

/** The AI section's status in the Settings nav, or `null` while unknown. */
export function aiStatus(preferences: Preferences | undefined): string | null {
  if (!preferences) return null
  if (!preferences.ai_enabled) return 'Off'
  return preferences.ai_provider === 'anthropic' ? 'On · Anthropic' : 'On · Ollama'
}

/** The Plex section's status in the Settings nav, or `null` while unknown. */
export function plexStatus(status: PlexStatus | undefined): string | null {
  if (!status) return null
  if (!status.configured) return 'Not set up'
  if (!status.connection) return 'Not checked yet'
  return status.connection.reachable ? 'Connected' : "Can't reach Plex"
}
