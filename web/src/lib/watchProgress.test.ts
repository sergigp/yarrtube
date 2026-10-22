import { describe, expect, it } from 'vitest'
import { watchProgress } from './watchProgress'

describe('watchProgress', () => {
  it('returns progress for an unwatched started video', () => {
    expect(watchProgress({ watched: false, position_seconds: 120, duration_seconds: 600 })).toEqual({
      percent: 20,
      secondsLeft: 480,
    })
  })
})
