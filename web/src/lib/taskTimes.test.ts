import { describe, expect, it } from 'vitest'
import { formatNextTry, formatRunTime, formatStartedAgo } from './taskTimes'

describe('formatRunTime', () => {
  const now = new Date('2026-11-14T20:03:00Z')
  const inUtc = { locale: 'en-GB', timeZone: 'UTC' }

  it('gives a clock time and the minutes left for a task due later today', () => {
    expect(formatRunTime('2026-11-14T20:20:00Z', now, inUtc)).toEqual({
      clock: '20:20',
      relative: 'in 17 min',
      due: false,
    })
  })

  it('gives hours and minutes left once over an hour away', () => {
    expect(formatRunTime('2026-11-14T22:13:00Z', now, inUtc).relative).toBe('in 2 h 10 min')
  })

  it('names tomorrow for a task due tomorrow', () => {
    expect(formatRunTime('2026-11-15T09:10:00Z', now, inUtc)).toEqual({
      clock: 'Tomorrow 09:10',
      relative: 'in 13 h 7 min',
      due: false,
    })
  })

  it('says starting soon once the task is due', () => {
    expect(formatRunTime('2026-11-14T20:00:00Z', now, inUtc)).toEqual({
      clock: '20:00',
      relative: 'Starting soon',
      due: true,
    })
  })
})

describe('formatStartedAgo', () => {
  const now = new Date('2026-11-14T20:03:00Z')

  it('says how long ago a task started', () => {
    expect(
      ['2026-11-14T20:02:48Z', '2026-11-14T19:58:00Z', '2026-11-14T17:00:00Z'].map((startedAt) =>
        formatStartedAgo(startedAt, now),
      ),
    ).toEqual(['started 12 s ago', 'started 5 min ago', 'started 3 h ago'])
  })
})

describe('formatNextTry', () => {
  const now = new Date('2026-11-14T20:03:00Z')
  const inUtc = { locale: 'en-GB', timeZone: 'UTC' }

  it('says Yarrtube tries again soon once due', () => {
    expect(formatNextTry('2026-11-14T20:00:00Z', now, inUtc)).toBe('Yarrtube tries again soon.')
  })

  it('gives the clock time and the time left when due later', () => {
    expect(formatNextTry('2026-11-14T20:09:00Z', now, inUtc)).toBe(
      'Yarrtube tries again at 20:09, in 6 min.',
    )
  })
})
