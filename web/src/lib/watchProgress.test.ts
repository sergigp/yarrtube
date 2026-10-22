import { describe, expect, it } from 'vitest'
import { watchProgress } from './watchProgress'

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
})
