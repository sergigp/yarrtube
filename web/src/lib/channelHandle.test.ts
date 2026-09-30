import { describe, expect, it } from 'vitest'
import { deriveChannelPathSegment } from './channelHandle'

describe('deriveChannelPathSegment', () => {
  it('returns an empty segment for empty or missing input', () => {
    expect(deriveChannelPathSegment('')).toBe('')
    expect(deriveChannelPathSegment('   ')).toBe('')
    expect(deriveChannelPathSegment(null)).toBe('')
    expect(deriveChannelPathSegment(undefined)).toBe('')
  })

  it('strips a leading @ from a bare handle', () => {
    expect(deriveChannelPathSegment('@somechannel')).toBe('somechannel')
    expect(deriveChannelPathSegment('somechannel')).toBe('somechannel')
  })

  it('takes the handle segment of a channel URL', () => {
    expect(deriveChannelPathSegment('https://youtube.com/@name')).toBe('name')
    expect(deriveChannelPathSegment('https://www.youtube.com/@name/videos')).toBe('name')
    expect(deriveChannelPathSegment('http://youtube.com/@name/featured')).toBe('name')
  })

  it('ignores query strings and fragments in URLs', () => {
    expect(deriveChannelPathSegment('https://youtube.com/@name?si=abc')).toBe('name')
    expect(deriveChannelPathSegment('https://youtube.com/@name#about')).toBe('name')
  })

  it('returns an empty segment for a URL without a path', () => {
    expect(deriveChannelPathSegment('https://youtube.com')).toBe('')
    expect(deriveChannelPathSegment('https://youtube.com/')).toBe('')
  })
})
