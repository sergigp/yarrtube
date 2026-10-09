import { describe, expect, it } from 'vitest'
import { channelSettingsChanges, lowersVideoLimit } from './channelSettings'

const current = { quality: 'high', video_limit: 10 } as const

describe('channelSettingsChanges', () => {
  it('returns nothing when no setting changed', () => {
    expect(channelSettingsChanges(current, { quality: 'high', video_limit: '10' })).toEqual({})
  })

  it('returns only a changed quality', () => {
    expect(channelSettingsChanges(current, { quality: 'low', video_limit: '10' })).toEqual({
      quality: 'low',
    })
  })

  it('returns only a changed video limit, as a number', () => {
    expect(channelSettingsChanges(current, { quality: 'high', video_limit: '25' })).toEqual({
      video_limit: 25,
    })
  })

  it('returns both settings when both changed', () => {
    expect(channelSettingsChanges(current, { quality: 'mid', video_limit: '3' })).toEqual({
      quality: 'mid',
      video_limit: 3,
    })
  })
})

describe('lowersVideoLimit', () => {
  it('is true for a limit below the current one', () => {
    expect(lowersVideoLimit(10, '3')).toBe(true)
  })

  it('is false for the same or a higher limit', () => {
    expect(lowersVideoLimit(10, '10')).toBe(false)
    expect(lowersVideoLimit(10, '20')).toBe(false)
  })

  it('is false while the field is empty', () => {
    expect(lowersVideoLimit(10, '')).toBe(false)
  })
})
