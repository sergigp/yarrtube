import { describe, expect, it } from 'vitest'
import { formatDuration } from './formatDuration'

describe('formatDuration', () => {
  it('returns null for missing or non-finite values', () => {
    expect(formatDuration(null)).toBeNull()
    expect(formatDuration(undefined)).toBeNull()
    expect(formatDuration(Number.NaN)).toBeNull()
    expect(formatDuration(Infinity)).toBeNull()
  })

  it('formats minutes and seconds', () => {
    expect(formatDuration(0)).toBe('0:00')
    expect(formatDuration(59)).toBe('0:59')
    expect(formatDuration(754)).toBe('12:34')
  })

  it('adds an hours component from one hour up', () => {
    expect(formatDuration(3600)).toBe('1:00:00')
    expect(formatDuration(3723)).toBe('1:02:03')
    expect(formatDuration(36000)).toBe('10:00:00')
  })

  it('rounds fractional seconds and clamps negatives to zero', () => {
    expect(formatDuration(89.6)).toBe('1:30')
    expect(formatDuration(-5)).toBe('0:00')
  })
})
