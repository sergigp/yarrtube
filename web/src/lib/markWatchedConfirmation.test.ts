import { describe, expect, it } from 'vitest'
import { markWatchedConfirmation } from './markWatchedConfirmation'

describe('markWatchedConfirmation', () => {
  it('asks nothing when at most one video is unwatched', () => {
    expect(markWatchedConfirmation('Foo', 0)).toBeNull()
    expect(markWatchedConfirmation('Foo', 1)).toBeNull()
  })
})
