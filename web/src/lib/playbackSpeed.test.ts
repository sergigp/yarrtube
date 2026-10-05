import { describe, expect, it } from 'vitest'
import { formatPlaybackSpeed, PLAYBACK_SPEEDS } from './playbackSpeed'

describe('PLAYBACK_SPEEDS', () => {
  it('offers 1x, 1.1x, 1.25x, 1.5x and 2x', () => {
    expect(PLAYBACK_SPEEDS).toEqual([1, 1.1, 1.25, 1.5, 2])
  })
})

describe('formatPlaybackSpeed', () => {
  it('formats a speed as its multiplier', () => {
    expect([1, 1.1, 1.75].map(formatPlaybackSpeed)).toEqual(['1x', '1.1x', '1.75x'])
  })
})
