import { describe, expect, it } from 'vitest'
import { formatTimeLeft, watchProgress } from './watchProgress'

describe('watchProgress', () => {
  it('returns progress for an unwatched started video', () => {
    expect(watchProgress({ watched: false, position_seconds: 120, duration_seconds: 600 })).toEqual({
      percent: 20,
      secondsLeft: 480,
    })
  })

  it('returns null for an unstarted, watched or durationless video', () => {
    expect(watchProgress({ watched: false, position_seconds: 0, duration_seconds: 600 })).toBeNull()
    expect(watchProgress({ watched: true, position_seconds: 120, duration_seconds: 600 })).toBeNull()
    expect(watchProgress({ watched: false, position_seconds: 120, duration_seconds: null })).toBeNull()
    expect(watchProgress({ watched: false, position_seconds: 120, duration_seconds: 0 })).toBeNull()
  })

  it('clamps a position past the duration', () => {
    expect(watchProgress({ watched: false, position_seconds: 700, duration_seconds: 600 })).toEqual({
      percent: 100,
      secondsLeft: 0,
    })
  })
})

describe('formatTimeLeft', () => {
  it('formats time left in minutes', () => {
    expect(formatTimeLeft(480)).toBe('8m left')
    expect(formatTimeLeft(539)).toBe('8m left')
  })

  it('formats time left under a minute', () => {
    expect(formatTimeLeft(45)).toBe('<1m left')
    expect(formatTimeLeft(0)).toBe('<1m left')
  })

  it('formats time left from one hour', () => {
    expect(formatTimeLeft(3600)).toBe('1h left')
    expect(formatTimeLeft(3900)).toBe('1h 5m left')
  })
})
