import { describe, expect, it } from 'vitest'
import { categoriesShown, summariesAvailable } from './preferences'
import { aPreferences } from '@/test/helpers'

describe('categoriesShown', () => {
  it('hides categories while the preferences are loading', () => {
    expect(categoriesShown(undefined, false)).toBe(false)
  })

  it('shows categories when the preferences failed to load', () => {
    expect(categoriesShown(undefined, true)).toBe(true)
  })

  it('follows the loaded preference', () => {
    expect(categoriesShown(aPreferences({ categories_enabled: false }), false)).toBe(false)
    expect(categoriesShown(aPreferences({ categories_enabled: true }), false)).toBe(true)
  })
})

describe('summariesAvailable', () => {
  it('is unavailable while the preferences are loading or failed', () => {
    expect(summariesAvailable(undefined)).toBe(false)
  })

  it('is available only with AI and summaries on', () => {
    const availability = [
      [false, false],
      [false, true],
      [true, false],
      [true, true],
    ].map(([ai_enabled, summaries_enabled]) =>
      summariesAvailable(aPreferences({ ai_enabled, summaries_enabled })),
    )

    expect(availability).toEqual([false, false, false, true])
  })
})
