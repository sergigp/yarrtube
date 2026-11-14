import { describe, expect, it } from 'vitest'
import { aiStatus } from './settingsStatus'
import { aPreferences } from '@/test/helpers'

describe('settingsStatus', () => {
  it('shows Off, On · Ollama and On · Anthropic for AI', () => {
    expect(aiStatus(aPreferences({ ai_enabled: false }))).toBe('Off')
    expect(aiStatus(aPreferences({ ai_enabled: true, ai_provider: 'ollama' }))).toBe('On · Ollama')
    expect(aiStatus(aPreferences({ ai_enabled: true, ai_provider: 'anthropic' }))).toBe(
      'On · Anthropic',
    )
    expect(aiStatus(undefined)).toBeNull()
  })
})
