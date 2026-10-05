import { describe, expect, it } from 'vitest'
import { PLAYBACK_SPEEDS } from './playbackSpeed'

describe('PLAYBACK_SPEEDS', () => {
  it('offers 1x, 1.1x, 1.25x, 1.5x and 2x', () => {
    expect(PLAYBACK_SPEEDS).toEqual([1, 1.1, 1.25, 1.5, 2])
  })
})
