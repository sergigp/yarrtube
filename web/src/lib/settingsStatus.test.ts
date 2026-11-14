import { describe, expect, it } from 'vitest'
import { aiStatus, plexStatus } from './settingsStatus'
import { aPlexConnection, aPlexStatus, aPreferences } from '@/test/helpers'

describe('settingsStatus', () => {
  it('shows Off, On · Ollama and On · Anthropic for AI', () => {
    expect(aiStatus(aPreferences({ ai_enabled: false }))).toBe('Off')
    expect(aiStatus(aPreferences({ ai_enabled: true, ai_provider: 'ollama' }))).toBe('On · Ollama')
    expect(aiStatus(aPreferences({ ai_enabled: true, ai_provider: 'anthropic' }))).toBe(
      'On · Anthropic',
    )
    expect(aiStatus(undefined)).toBeNull()
  })

  it("shows Not set up, Not checked yet, Connected and Can't reach Plex", () => {
    expect(plexStatus(aPlexStatus({ configured: false }))).toBe('Not set up')
    expect(plexStatus(aPlexStatus({ connection: null }))).toBe('Not checked yet')
    expect(plexStatus(aPlexStatus({ connection: aPlexConnection({ reachable: true }) }))).toBe(
      'Connected',
    )
    expect(
      plexStatus(aPlexStatus({ connection: aPlexConnection({ reachable: false, error: 'refused' }) })),
    ).toBe("Can't reach Plex")
    expect(plexStatus(undefined)).toBeNull()
  })
})
