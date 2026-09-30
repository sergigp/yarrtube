import { describe, expect, it } from 'vitest'
import { slugify } from './slugify'

describe('slugify', () => {
  it('lowercases and joins words with single dashes', () => {
    expect(slugify('My Channel Name')).toBe('my-channel-name')
  })

  it('collapses runs of non-alphanumeric characters', () => {
    expect(slugify('Rock & Roll!!! (live)')).toBe('rock-roll-live')
  })

  it('trims leading and trailing dashes', () => {
    expect(slugify('  --Hello--  ')).toBe('hello')
  })

  it('returns an empty slug for empty or missing input', () => {
    expect(slugify('')).toBe('')
    expect(slugify(null)).toBe('')
    expect(slugify(undefined)).toBe('')
    expect(slugify('!!!')).toBe('')
  })
})
