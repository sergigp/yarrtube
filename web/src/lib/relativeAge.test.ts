import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { formatAge } from './relativeAge'

describe('formatAge', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-10-30T12:00:00Z'))
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('reads as the largest whole unit: hours, days, weeks, months then years', () => {
    expect(
      [
        '2026-10-30T11:00:00Z',
        '2026-10-29T12:00:01Z',
        '2026-10-29T12:00:00Z',
        '2026-10-23T12:00:01Z',
        '2026-10-23T12:00:00Z',
        '2026-10-01T12:00:00Z',
        '2026-09-30T12:00:00Z',
        '2025-10-31T12:00:00Z',
        '2025-10-30T12:00:00Z',
        '2023-06-01T12:00:00Z',
      ].map((published) => formatAge(published)),
    ).toEqual([
      '1h ago',
      '23h ago',
      '1d ago',
      '6d ago',
      '1w ago',
      '4w ago',
      '1mo ago',
      '12mo ago',
      '1y ago',
      '3y ago',
    ])
  })

  it('reads anything under an hour, or ahead of now, as just now', () => {
    expect([formatAge('2026-10-30T11:30:00Z'), formatAge('2026-10-30T13:00:00Z')]).toEqual([
      'just now',
      'just now',
    ])
  })

  it('has no age for an unknown or unreadable date', () => {
    expect([formatAge(null), formatAge('garbage')]).toEqual([null, null])
  })
})
