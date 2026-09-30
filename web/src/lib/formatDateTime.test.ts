import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { formatDate, formatDateTime, formatRelativeTime } from './formatDateTime'

describe('formatDateTime', () => {
  it('returns a placeholder for a missing timestamp', () => {
    expect(formatDateTime(null)).toBe('—')
    expect(formatDateTime('')).toBe('—')
  })

  it('returns the raw string when it does not parse as a date', () => {
    expect(formatDateTime('not-a-date')).toBe('not-a-date')
  })

  it('formats a valid timestamp with date and time but no seconds', () => {
    const formatted = formatDateTime('2026-03-05T14:30:45Z')
    expect(formatted).toContain('2026')
    expect(formatted).not.toContain(':45')
  })
})

describe('formatDate', () => {
  it('returns a placeholder for a missing timestamp', () => {
    expect(formatDate(null)).toBe('—')
    expect(formatDate('')).toBe('—')
  })

  it('returns the raw string when it does not parse as a date', () => {
    expect(formatDate('garbage')).toBe('garbage')
  })

  it('formats a valid timestamp without a time of day', () => {
    const formatted = formatDate('2026-03-05T14:30:45Z')
    expect(formatted).toContain('2026')
    expect(formatted).not.toContain('14')
    expect(formatted).not.toContain('30')
  })
})

describe('formatRelativeTime', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-06-15T12:00:00Z'))
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('returns a placeholder for a missing timestamp', () => {
    expect(formatRelativeTime(null)).toBe('—')
    expect(formatRelativeTime('')).toBe('—')
  })

  it('returns the raw string when it does not parse as a date', () => {
    expect(formatRelativeTime('garbage')).toBe('garbage')
  })

  it('treats anything within five seconds as just now', () => {
    expect(formatRelativeTime('2026-06-15T12:00:00Z')).toBe('just now')
    expect(formatRelativeTime('2026-06-15T11:59:56Z')).toBe('just now')
    expect(formatRelativeTime('2026-06-15T12:00:04Z')).toBe('just now')
  })

  it('formats past times with the largest fitting unit', () => {
    expect(formatRelativeTime('2026-06-15T11:59:50Z')).toBe('10s ago')
    expect(formatRelativeTime('2026-06-15T11:58:00Z')).toBe('2m ago')
    expect(formatRelativeTime('2026-06-15T09:00:00Z')).toBe('3h ago')
    expect(formatRelativeTime('2026-06-13T12:00:00Z')).toBe('2d ago')
    expect(formatRelativeTime('2026-04-15T12:00:00Z')).toBe('2mo ago')
    expect(formatRelativeTime('2024-06-15T12:00:00Z')).toBe('2y ago')
  })

  it('formats future times with an "in" prefix', () => {
    expect(formatRelativeTime('2026-06-15T12:00:30Z')).toBe('in 30s')
    expect(formatRelativeTime('2026-06-15T14:00:00Z')).toBe('in 2h')
  })
})
