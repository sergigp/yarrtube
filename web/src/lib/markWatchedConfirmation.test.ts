import { describe, expect, it } from 'vitest'
import { markWatchedConfirmation } from './markWatchedConfirmation'

describe('markWatchedConfirmation', () => {
  it('names how many videos it marks and that it cannot be undone in bulk', () => {
    expect(markWatchedConfirmation('Foo', 23)).toEqual({
      title: 'Mark 23 videos in "Foo" watched?',
      description: "This can't be undone in bulk: videos can only be marked unwatched one by one.",
    })
  })

  it('asks even for one video, or none known unwatched', () => {
    expect([
      markWatchedConfirmation('Foo', 1).title,
      markWatchedConfirmation('Foo', 0).title,
    ]).toEqual(['Mark 1 video in "Foo" watched?', 'Mark everything in "Foo" watched?'])
  })
})
