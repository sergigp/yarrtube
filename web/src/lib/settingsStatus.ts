import type { PlexStatus, Preferences } from '@/api/types'

/** The AI section's status in the Settings nav, or `null` while unknown. */
export function aiStatus(preferences: Preferences | undefined): string | null {
  if (!preferences) return null
  if (!preferences.ai_enabled) return 'Off'
  return preferences.ai_provider === 'anthropic' ? 'On · Anthropic' : 'On · Ollama'
}

/** The Plex section's status in the Settings nav, or `null` while unknown. */
export function plexStatus(_status: PlexStatus | undefined): string | null {
  return null
}
