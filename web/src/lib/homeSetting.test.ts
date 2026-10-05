import { describe, expect, it } from 'vitest'
import { homeSettingAction } from './homeSetting'

describe('homeSettingAction', () => {
  it('describes excluding a playlist from home', () => {
    expect(homeSettingAction(true, 'Bluey')).toBe('exclude "Bluey" from home')
  })

  it('describes including a playlist in home', () => {
    expect(homeSettingAction(false, 'Bluey')).toBe('include "Bluey" in home')
  })
})
