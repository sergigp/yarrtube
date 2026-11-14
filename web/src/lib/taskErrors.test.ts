import { describe, expect, it } from 'vitest'
import { explainTaskError } from './taskErrors'

describe('explainTaskError', () => {
  it.each([
    ['ERROR: unable to download video data: HTTP Error 403: Forbidden', 'YouTube refused the request (403)'],
    ['ERROR: [youtube] abc: Private video. Sign in if you have access', 'The video is private'],
    ['ERROR: [youtube] abc: Sign in to confirm your age. This video may be inappropriate', 'YouTube only shows the video to signed-in adults'],
    ['ERROR: [youtube] abc: Join this channel to get access to members-only content', 'The video is for channel members only'],
    ['ERROR: [youtube] abc: Video unavailable. This video has been removed', 'YouTube says the video is unavailable'],
  ])('explains %s in plain words', (raw, plain) => {
    expect(explainTaskError(raw)).toBe(plain)
  })

  it('falls back to the first line without its ERROR: prefix', () => {
    expect(explainTaskError('ERROR: something odd happened\nTraceback (most recent call last)')).toBe(
      'something odd happened',
    )
  })
})
