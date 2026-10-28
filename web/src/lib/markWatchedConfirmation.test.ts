import { describe, expect, it } from 'vitest'
import { markWatchedConfirmation } from './markWatchedConfirmation'

describe('markWatchedConfirmation', () => {
  it('asks nothing when at most one video is unwatched', () => {
    expect(markWatchedConfirmation('Foo', 0)).toBeNull()
    expect(markWatchedConfirmation('Foo', 1)).toBeNull()
  })

  it('names how many videos it marks and that it cannot be undone in bulk', () => {
    expect(markWatchedConfirmation('Foo', 23)).toEqual({
      title: 'Mark 23 videos in "Foo" watched?',
      description: "This can't be undone in bulk: videos can only be marked unwatched one by one.",
    })
  })
})
