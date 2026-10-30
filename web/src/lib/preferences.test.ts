import { describe, expect, it } from 'vitest'
import { categoriesShown, homeTintShown } from './preferences'
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

describe('homeTintShown', () => {
  it('tints only with both categories and category colors on', () => {
    expect(homeTintShown(aPreferences(), false)).toBe(true)
    expect(homeTintShown(aPreferences({ home_show_category_colors: false }), false)).toBe(false)
    expect(homeTintShown(aPreferences({ categories_enabled: false }), false)).toBe(false)
  })

  it('does not tint while the preferences are loading', () => {
    expect(homeTintShown(undefined, false)).toBe(false)
  })

  it('tints when the preferences failed to load', () => {
    expect(homeTintShown(undefined, true)).toBe(true)
  })
})
