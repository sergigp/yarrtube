import { describe, expect, it } from 'vitest'
import { categoriesShown } from './preferences'
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
